//! Durable provider handoffs, fenced claims and immutable retry evidence.

#[path = "delivery_state.rs"]
mod state;
#[path = "delivery_codec.rs"]
mod values;
use super::{audit, authorization, plan, port, preferences, records, stored, PostgresAlertStore};
use application::{alerts::*, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher};
use postgres::{GenericClient, Transaction};
use state::{Claim, State};
use time::Duration;
use uuid::Uuid;

const TECHNICAL_ACTOR: &str = "system:activity-alerts";

pub(super) fn enqueue(
    tx: &mut Transaction<'_>,
    id: AlertId,
    now: OffsetDateTime,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    if let Some(row) = tx
        .query_opt(
            "SELECT id FROM alert_email_outbox WHERE alert_id=$1",
            &[&id.as_uuid()],
        )
        .map_err(port)?
    {
        state::load(tx, row.try_get("id").map_err(stored)?, hasher)?;
        return Ok(());
    }
    let record = records::load(tx, id, hasher)?;
    record.validate(record.recipient_id, now)?;
    let initial = State {
        id: Uuid::new_v4(),
        alert: id,
        sequence: 0,
        status: "pending".into(),
        attempt: 0,
        at: now,
        next: Some(now),
        first: None,
        claim: None,
        message: None,
        outcome: None,
        uncertain: false,
    };
    state::write(tx, &initial, hasher)
}

pub(super) fn status(
    tx: &mut Transaction<'_>,
    id: AlertId,
    hasher: &dyn DocumentHasher,
) -> Result<AlertEmailStatus, ApplicationError> {
    match tx
        .query_opt(
            "SELECT id FROM alert_email_outbox WHERE alert_id=$1",
            &[&id.as_uuid()],
        )
        .map_err(port)?
    {
        Some(row) => state::load(tx, row.try_get("id").map_err(stored)?, hasher)?.email_status(),
        None => {
            if records::load(tx, id, hasher)?.email != AlertEmailStatus::Disabled {
                return Err(stored("enabled notification delivery absent"));
            }
            Ok(AlertEmailStatus::Disabled)
        }
    }
}

pub(super) fn validate<C: GenericClient>(
    tx: &mut C,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let mut after: Option<Uuid> = None;
    loop {
        let rows=tx.query("SELECT id FROM alert_email_outbox WHERE $1::uuid IS NULL OR id>$1 ORDER BY id LIMIT 64",&[&after]).map_err(port)?;
        for row in &rows {
            let id = row.try_get("id").map_err(stored)?;
            state::load(tx, id, hasher)?;
            after = Some(id);
        }
        if rows.len() < 64 {
            return Ok(());
        }
    }
}

fn save(
    tx: &mut Transaction<'_>,
    entry: &mut State,
    now: OffsetDateTime,
    hasher: &dyn DocumentHasher,
    action: &str,
) -> Result<(), ApplicationError> {
    if now < entry.at {
        return Err(stored("delivery clock regressed"));
    }
    entry.sequence = entry
        .sequence
        .checked_add(1)
        .ok_or_else(|| stored("delivery sequence exhausted"))?;
    entry.at = now;
    state::write(tx, entry, hasher)?;
    state::load(tx, entry.id, hasher)?;
    audit(
        tx,
        TECHNICAL_ACTOR,
        action,
        &format!("delivery:{}:sequence:{}", entry.id, entry.sequence),
        now,
    )
}

fn stop_delivery(
    tx: &mut Transaction<'_>,
    entry: &mut State,
    now: OffsetDateTime,
    hasher: &dyn DocumentHasher,
    status: &str,
) -> Result<(), ApplicationError> {
    entry.status = if entry.uncertain { "unknown" } else { status }.into();
    entry.next = None;
    save(tx, entry, now, hasher, "alert.email_stopped")
}

impl AlertDeliveryStore for PostgresAlertStore {
    fn claim_next(&self) -> Result<Option<AlertDeliveryClaim>, ApplicationError> {
        let Some(configuration) = &self.email else {
            return Ok(None);
        };
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let now = self.now()?;
        let rows = tx
            .query(
                "SELECT id FROM alert_email_outbox WHERE status IN ('pending','unknown','sending')
            AND next_seconds IS NOT NULL AND (next_seconds,next_nanos)<=($1,$2)
            ORDER BY next_seconds,next_nanos,id LIMIT 100",
                &[&now.unix_timestamp(), &(now.nanosecond() as i32)],
            )
            .map_err(port)?;
        for row in rows {
            let mut entry = state::load(
                &mut tx,
                row.try_get("id").map_err(stored)?,
                self.hasher.as_ref(),
            )?;
            if now < entry.at {
                return Err(stored("delivery clock regressed"));
            }
            if entry.status == "sending" {
                entry.uncertain = true;
            }
            if !entry.retryable_at(now) {
                stop_delivery(&mut tx, &mut entry, now, self.hasher.as_ref(), "failed")?;
                continue;
            }
            let mut record = records::load(&mut tx, entry.alert, self.hasher.as_ref())?;
            let current = records::current(&mut tx, &mut record, self.hasher.as_ref(), now)?;
            record.validate(record.recipient_id, now)?;
            let recipient = authorization::recipient(
                &mut tx,
                record.subject,
                record.recipient_id,
                current.responsible,
            )?;
            let prefs = preferences::load(
                &mut tx,
                record.recipient_id,
                self.hasher.as_ref(),
                self.transport(),
            )?;
            let enabled = plan::channels(record.kind, &prefs.values, record.subject).email;
            if record.state != AlertState::Active || recipient.is_none() || !enabled {
                stop_delivery(&mut tx, &mut entry, now, self.hasher.as_ref(), "cancelled")?;
                continue;
            }
            let recipient =
                recipient.ok_or_else(|| stored("eligible delivery recipient absent"))?;
            if entry
                .message
                .as_ref()
                .is_some_and(|message| message.recipient_email != recipient.email)
            {
                stop_delivery(&mut tx, &mut entry, now, self.hasher.as_ref(), "cancelled")?;
                continue;
            }
            if entry.message.is_none() {
                entry.message = Some(AlertEmailMessage {
                    idempotency_key: format!("alert-email:{}", entry.id),
                    from_email: configuration.from_email.clone(),
                    recipient_email: recipient.email,
                    login_url: configuration.login_url.clone(),
                    template: AlertEmailTemplate::GenericLoginV1,
                });
            }
            entry.first = Some(entry.first.unwrap_or(now));
            entry.attempt += 1;
            entry.claim = Some(Claim {
                id: Uuid::new_v4(),
                at: now,
                until: state::after(now, Duration::seconds(120))
                    .ok_or_else(|| stored("delivery lease exceeds supported time"))?,
            });
            entry.status = "sending".into();
            entry.outcome = None;
            entry.next = entry.claim.as_ref().map(|claim| claim.until);
            save(
                &mut tx,
                &mut entry,
                now,
                self.hasher.as_ref(),
                "alert.email_claimed",
            )?;
            let result = entry.public()?;
            tx.commit().map_err(port)?;
            return Ok(Some(result));
        }
        tx.commit().map_err(port)?;
        Ok(None)
    }

    fn complete_attempt(
        &self,
        completion: AlertDeliveryCompletion,
    ) -> Result<(), ApplicationError> {
        values::validate_outcome(&completion.outcome)?;
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let mut entry = state::load(
            &mut tx,
            completion.delivery_id.as_uuid(),
            self.hasher.as_ref(),
        )?;
        if entry.attempt != completion.attempt
            || entry
                .claim
                .as_ref()
                .is_none_or(|claim| claim.id != completion.claim_id.as_uuid())
        {
            return Err(AlertError::OperationConflict.into());
        }
        if entry.status != "sending" {
            if entry.outcome.as_ref() == Some(&completion.outcome) {
                tx.commit().map_err(port)?;
                return Ok(());
            }
            return Err(AlertError::OperationConflict.into());
        }
        let now = self.now()?;
        if now < entry.at {
            return Err(stored("delivery completion clock regressed"));
        }
        let delay = 30_i64
            .checked_mul(1_i64 << entry.attempt.saturating_sub(1))
            .unwrap_or(3600)
            .min(3600);
        let retry_at = state::after(now, Duration::seconds(delay));
        entry.next = None;
        match &completion.outcome {
            AlertEmailOutcome::Accepted { .. } => {
                entry.status = "accepted".into();
                entry.uncertain = false;
            }
            AlertEmailOutcome::Permanent { .. } => {
                entry.status = if entry.uncertain { "unknown" } else { "failed" }.into()
            }
            AlertEmailOutcome::Retryable { .. } | AlertEmailOutcome::Unknown { .. } => {
                entry.uncertain |= matches!(completion.outcome, AlertEmailOutcome::Unknown { .. });
                let retry = retry_at.is_some_and(|at| entry.retryable_at(at));
                entry.status = if entry.uncertain {
                    "unknown"
                } else if retry {
                    "pending"
                } else {
                    "failed"
                }
                .into();
                if retry {
                    entry.next = retry_at;
                }
            }
        }
        entry.outcome = Some(completion.outcome);
        save(
            &mut tx,
            &mut entry,
            now,
            self.hasher.as_ref(),
            "alert.email_result",
        )?;
        tx.commit().map_err(port)
    }
}
