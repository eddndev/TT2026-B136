use super::{authorization, decode, inconsistent, port, storage, PostgresHearingResultStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{hearing_results::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, hearings::HearingId, identity::UserId};

impl PostgresHearingResultStore {
    pub(super) fn list_page(
        &self,
        actor: UserId,
        case: CaseId,
        hearing: HearingId,
        query: HearingResultQuery,
        at: OffsetDateTime,
    ) -> Result<HearingResultPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        tx.query_opt(
            "SELECT id FROM case_hearings WHERE id=$1 AND case_id=$2",
            &[&hearing.as_uuid(), &case.as_uuid()],
        )
        .map_err(port)?
        .ok_or(HearingResultError::ReferenceNotFound)?;
        let after = query.after_id().map(|id| id.as_uuid());
        let status = query.status().status().map(|s| s.as_str());
        let limit = i64::from(query.limit()) + 1;
        let mut rows=tx.query("SELECT result_id,revision FROM (SELECT DISTINCT ON (result_id) result_id,revision,status
            FROM case_hearing_result_revisions WHERE case_id=$1 AND hearing_id=$2 ORDER BY result_id,revision DESC) heads
            WHERE ($3::uuid IS NULL OR result_id>$3) AND ($4::text IS NULL OR status=$4) ORDER BY result_id LIMIT $5",
            &[&case.as_uuid(),&hearing.as_uuid(),&after,&status,&limit]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let id = HearingResultId::from_uuid(row.get(0));
            let revision =
                HearingResultRevision::new(decode::counter(row.get(1))?).map_err(inconsistent)?;
            let detail = storage::detail(
                &mut tx,
                case,
                hearing,
                id,
                Some(revision),
                self.hasher.as_ref(),
            )?;
            let s = detail.snapshot;
            results.push(HearingResultOverview {
                case_id: case,
                hearing_id: hearing,
                id: s.id,
                revision: s.revision,
                status: s.status,
                occurrence: s.values.occurrence(),
                extent: s.values.extent(),
                event_time: s.values.event_time(),
                attendee_count: s.values.attendees().len() as u8,
                agreement_count: s.values.agreements().len() as u8,
                anchor_revision: s.anchor.revision,
            });
        }
        let next_after_id = if has_more {
            results.last().map(|r| r.id)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "hearing_result.list_read",
            &format!("case:{case}:hearing:{hearing}:results"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(HearingResultPage {
            results,
            has_more,
            next_after_id,
        })
    }
    pub(super) fn history_page(
        &self,
        actor: UserId,
        case: CaseId,
        hearing: HearingId,
        id: HearingResultId,
        query: HearingResultHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<HearingResultHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        tx.query_opt(
            "SELECT id FROM case_hearing_results WHERE id=$1 AND case_id=$2 AND hearing_id=$3",
            &[&id.as_uuid(), &case.as_uuid(), &hearing.as_uuid()],
        )
        .map_err(port)?
        .ok_or(HearingResultError::NotFound)?;
        let before = query.before_revision().map(|v| i64::from(v.get()));
        let limit = i64::from(query.limit()) + 1;
        let mut rows=tx.query("SELECT revision FROM case_hearing_result_revisions WHERE result_id=$1 AND case_id=$2 AND hearing_id=$3
            AND ($4::bigint IS NULL OR revision<$4) ORDER BY revision DESC LIMIT $5",&[&id.as_uuid(),&case.as_uuid(),&hearing.as_uuid(),&before,&limit]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut revisions = Vec::with_capacity(rows.len());
        for row in rows {
            let revision =
                HearingResultRevision::new(decode::counter(row.get(0))?).map_err(inconsistent)?;
            let detail = storage::detail(
                &mut tx,
                case,
                hearing,
                id,
                Some(revision),
                self.hasher.as_ref(),
            )?;
            revisions.push(HearingResultHistoryEntry::from(&detail.snapshot));
        }
        let next_before_revision = if has_more {
            revisions.last().map(|r| r.revision)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "hearing_result.history_read",
            &format!("case:{case}:hearing:{hearing}:result:{id}:history"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(HearingResultHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
}
