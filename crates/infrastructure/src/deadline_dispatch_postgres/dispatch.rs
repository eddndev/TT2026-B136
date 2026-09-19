use super::{cursor, events, inconsistent, jobs, port, selection, PostgresDeadlineDispatchStore};
use application::{deadline_dispatch::*, ApplicationError};
use time::UtcOffset;

impl DeadlineDispatchStore for PostgresDeadlineDispatchStore {
    fn dispatch(
        &self,
        request: DeadlineDispatchRequest,
    ) -> Result<DeadlineDispatchBatch, ApplicationError> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| inconsistent("dispatch database lock poisoned"))?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let old = cursor::load(&mut tx)?;
        let mut progress = old;
        let event = match request.stream {
            DeadlineDispatchStream::Events => events::load(&mut tx, old.event)?,
            DeadlineDispatchStream::LegacyBootstrap => None,
        };
        if request.stream == DeadlineDispatchStream::Events && event.is_none() {
            tx.commit().map_err(port)?;
            return Ok(DeadlineDispatchBatch {
                stream: request.stream,
                event: None,
                selected: 0,
                inserted: 0,
                completed_scan: false,
                progress,
            });
        }
        let after = match request.stream {
            DeadlineDispatchStream::Events => old.event.after_deadline_id,
            DeadlineDispatchStream::LegacyBootstrap => old.bootstrap_after_deadline_id,
        };
        let mut candidates = selection::page(
            &mut tx,
            event.map(|value| value.sequence),
            after,
            request.limit,
        )?;
        let partial =
            candidates.len() > usize::try_from(request.limit.get()).map_err(inconsistent)?;
        if partial {
            candidates.pop();
        }
        let selected = u32::try_from(candidates.len()).map_err(inconsistent)?;
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&at.year()) {
            return Err(inconsistent("dispatch clock is outside supported years"));
        }
        let mut inserted = 0_u32;
        for candidate in &candidates {
            // Verify each capture without retaining all historical material in the page.
            crate::deadline_postgres::storage::detail(
                &mut tx,
                candidate.case_id,
                candidate.id,
                None,
                self.hasher.as_ref(),
            )?;
            if jobs::insert(&mut tx, *candidate, event.map(|value| value.sequence), at)? {
                inserted += 1;
            }
        }
        let last = if partial {
            candidates.last().map(|value| value.id)
        } else {
            None
        };
        match request.stream {
            DeadlineDispatchStream::Events => {
                let sequence = event
                    .ok_or_else(|| inconsistent("event stream lost its source"))?
                    .sequence;
                if partial {
                    progress.event.active_sequence = Some(sequence);
                    progress.event.after_deadline_id = last;
                } else {
                    progress.event = DeadlineEventDispatchPosition {
                        completed_sequence: Some(sequence),
                        active_sequence: None,
                        after_deadline_id: None,
                    };
                }
            }
            DeadlineDispatchStream::LegacyBootstrap => {
                progress.bootstrap_after_deadline_id = last;
            }
        }
        if progress != old {
            cursor::save(&mut tx, progress)?;
        }
        if progress != old || inserted != 0 {
            let stream = match request.stream {
                DeadlineDispatchStream::Events => "events",
                DeadlineDispatchStream::LegacyBootstrap => "legacy_bootstrap",
            };
            crate::audit_postgres::append_transaction(&mut tx, "deadline_reevaluator", "deadline.dispatch_advanced",
                &format!("stream:{stream}:event:{:?}:policy:1:selected:{selected}:inserted:{inserted}:before:{old:?}:after:{progress:?}", event.map(|value| value.sequence)), at)?;
        }
        tx.commit().map_err(port)?;
        Ok(DeadlineDispatchBatch {
            stream: request.stream,
            event,
            selected,
            inserted,
            completed_scan: !partial,
            progress,
        })
    }
}
