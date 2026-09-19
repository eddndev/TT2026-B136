use super::super::{codec, port, stored};
use application::{alerts::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::GenericClient;
use serde_json::{json, Value};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub(super) const MAX_ATTEMPTS: u32 = 8;
pub(super) const WINDOW_HOURS: i64 = 23;
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Claim {
    pub id: Uuid,
    pub at: OffsetDateTime,
    pub until: OffsetDateTime,
}
#[derive(Clone)]
pub(super) struct State {
    pub id: Uuid,
    pub alert: AlertId,
    pub sequence: i64,
    pub status: String,
    pub attempt: u32,
    pub at: OffsetDateTime,
    pub next: Option<OffsetDateTime>,
    pub first: Option<OffsetDateTime>,
    pub claim: Option<Claim>,
    pub message: Option<AlertEmailMessage>,
    pub outcome: Option<AlertEmailOutcome>,
    pub uncertain: bool,
}
impl State {
    pub fn encode(&self) -> Value {
        json!({"id":self.id,"alert":self.alert.as_uuid(),"sequence":self.sequence,"status":self.status,
            "attempt":self.attempt,"at":codec::instant(self.at),"next":codec::optional(self.next),
            "first":codec::optional(self.first),"uncertain":self.uncertain,
            "claim":self.claim.as_ref().map(|c|json!({"id":c.id,"at":codec::instant(c.at),"until":codec::instant(c.until)})),
            "message":self.message.as_ref().map(super::values::message),"outcome":self.outcome.as_ref().map(super::values::outcome)})
    }
    pub fn public(&self) -> Result<AlertDeliveryClaim, ApplicationError> {
        let claim = self
            .claim
            .as_ref()
            .ok_or_else(|| stored("delivery claim absent"))?;
        Ok(AlertDeliveryClaim {
            delivery_id: AlertDeliveryId::from_uuid(self.id),
            alert_id: self.alert,
            claim_id: AlertClaimId::from_uuid(claim.id),
            attempt: self.attempt,
            claimed_at: claim.at,
            lease_until: claim.until,
            first_attempt_at: self
                .first
                .ok_or_else(|| stored("delivery first attempt absent"))?,
            message: self
                .message
                .clone()
                .ok_or_else(|| stored("delivery message absent"))?,
        })
    }
    pub fn email_status(&self) -> Result<AlertEmailStatus, ApplicationError> {
        Ok(match self.status.as_str() {
            "pending" => AlertEmailStatus::Pending,
            "sending" => AlertEmailStatus::Sending,
            "accepted" => AlertEmailStatus::Accepted {
                accepted_at: self.at,
            },
            "failed" => AlertEmailStatus::Failed,
            "unknown" => AlertEmailStatus::Unknown,
            "cancelled" => AlertEmailStatus::Cancelled,
            "disabled" => AlertEmailStatus::Disabled,
            _ => return Err(stored("invalid delivery state")),
        })
    }
    pub fn retryable_at(&self, now: OffsetDateTime) -> bool {
        self.attempt < MAX_ATTEMPTS
            && self.first.is_none_or(|first| {
                after(first, Duration::hours(WINDOW_HOURS)).is_some_and(|end| now < end)
            })
    }
}
pub(super) fn load<C: GenericClient>(
    tx: &mut C,
    id: Uuid,
    hasher: &dyn DocumentHasher,
) -> Result<State, ApplicationError> {
    let row = tx
        .query_opt("SELECT * FROM alert_email_outbox WHERE id=$1", &[&id])
        .map_err(port)?
        .ok_or_else(|| stored("delivery absent"))?;
    let head = super::values::decode(&row, hasher)?;
    if head.id != id
        || head.alert.as_uuid() != row.try_get::<_, Uuid>("alert_id").map_err(stored)?
        || head.sequence != row.try_get::<_, i64>("sequence").map_err(stored)?
        || head.status != row.try_get::<_, String>("status").map_err(stored)?
        || head.next != codec::row_time(&row, "next_seconds", "next_nanos")?
    {
        return Err(stored("delivery index differs"));
    }
    let rows = tx
        .query(
            "SELECT * FROM alert_email_attempts WHERE delivery_id=$1 ORDER BY sequence LIMIT 34",
            &[&id],
        )
        .map_err(port)?;
    if rows.len() != head.sequence as usize + 1 {
        return Err(stored("delivery ledger incomplete"));
    }
    let mut previous: Option<State> = None;
    for (index, row) in rows.iter().enumerate() {
        let entry = super::values::decode(row, hasher)?;
        if entry.id != id
            || entry.alert != head.alert
            || entry.sequence != index as i64
            || row.try_get::<_, i64>("sequence").map_err(stored)? != entry.sequence
        {
            return Err(stored("delivery ledger identity differs"));
        }
        if let Some(prior) = previous {
            if entry.at < prior.at
                || entry.attempt < prior.attempt
                || entry.attempt > prior.attempt + 1
                || prior.message.is_some() && entry.message != prior.message
                || prior.first.is_some() && entry.first != prior.first
            {
                return Err(stored("delivery history regressed or message changed"));
            }
            if entry.status == "sending" {
                if !matches!(prior.status.as_str(), "pending" | "unknown" | "sending")
                    || prior.next.is_none_or(|at| at > entry.at)
                    || entry.attempt != prior.attempt + 1
                    || entry.claim == prior.claim
                    || entry.outcome.is_some()
                {
                    return Err(stored("delivery claim transition differs"));
                }
            } else {
                if entry.attempt != prior.attempt || entry.claim != prior.claim {
                    return Err(stored("delivery completion fencing differs"));
                }
                if prior.status != "sending"
                    && (entry.next.is_some()
                        || entry.outcome != prior.outcome
                        || !matches!(entry.status.as_str(), "failed" | "cancelled" | "unknown"))
                {
                    return Err(stored("unclaimed delivery outcome changed"));
                }
                if prior.status == "sending"
                    && entry.outcome.is_none()
                    && (entry.status != "unknown" || entry.next.is_some())
                {
                    return Err(stored("lost provider response was not preserved"));
                }
                if prior.uncertain && !entry.uncertain && entry.status != "accepted" {
                    return Err(stored("delivery uncertainty regressed"));
                }
            }
            if matches!(
                prior.status.as_str(),
                "accepted" | "failed" | "cancelled" | "disabled"
            ) || prior.next.is_none()
            {
                return Err(stored("terminal delivery changed"));
            }
        } else if entry.sequence != 0
            || entry.attempt != 0
            || entry.status != "pending"
            || entry.next != Some(entry.at)
            || entry.uncertain
            || entry.outcome.is_some()
        {
            return Err(stored("initial delivery differs"));
        }
        previous = Some(entry);
    }
    if previous.is_none_or(|last| last.encode() != head.encode()) {
        return Err(stored("delivery projection differs from ledger"));
    }
    Ok(head)
}
pub(super) fn write<C: GenericClient>(
    tx: &mut C,
    state: &State,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let (bytes, digest) = codec::payload(&state.encode(), hasher)?;
    let seconds = state.next.map(|at| at.unix_timestamp());
    let nanos = state.next.map(|at| at.nanosecond() as i32);
    if state.sequence == 0 {
        tx.execute("INSERT INTO alert_email_outbox(id,alert_id,status,next_seconds,next_nanos,sequence,payload,payload_digest) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",&[&state.id,&state.alert.as_uuid(),&state.status,&seconds,&nanos,&state.sequence,&bytes,&digest]).map_err(port)?;
    } else {
        let updated=tx.execute("UPDATE alert_email_outbox SET status=$2,next_seconds=$3,next_nanos=$4,sequence=$5,payload=$6,payload_digest=$7 WHERE id=$1 AND sequence=$8",&[&state.id,&state.status,&seconds,&nanos,&state.sequence,&bytes,&digest,&(state.sequence-1)]).map_err(port)?;
        if updated != 1 {
            return Err(stored("delivery sequence changed"));
        }
    }
    tx.execute("INSERT INTO alert_email_attempts(delivery_id,sequence,payload,payload_digest) VALUES($1,$2,$3,$4)",&[&state.id,&state.sequence,&bytes,&digest]).map_err(port)?;
    Ok(())
}

pub(super) fn after(at: OffsetDateTime, duration: Duration) -> Option<OffsetDateTime> {
    at.checked_add(duration)
        .filter(|value| (1..=9999).contains(&value.year()))
}
