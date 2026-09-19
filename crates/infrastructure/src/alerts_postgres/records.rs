use super::{codec, port, stored, subject};
use application::{alerts::*, ApplicationError};
use domain::{crypto::DocumentHasher, identity::UserId};
use postgres::{Row, Transaction};
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) fn encode(record: &AlertRecord) -> Value {
    json!({"id":record.id.as_uuid(),"recipient":record.recipient_id.as_uuid(),"occurrence":record.occurrence_id.as_uuid(),
        "subject":codec::subject(record.subject),"kind":codec::kind(record.kind),"origin":codec::origin(record.origin),
        "trigger":codec::instant(record.trigger_at),"created":codec::instant(record.created_at),
        "subject_title":record.subject_title,"case_title":record.case_title,"case_reference":record.case_reference,
        "email":if record.email==AlertEmailStatus::Disabled {"disabled"} else {"pending"}})
}
pub(super) fn decode(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<AlertRecord, ApplicationError> {
    let value = codec::read(row, hasher)?;
    let mut result = AlertRecord {
        id: AlertId::from_uuid(codec::uuid(&value["id"])?),
        recipient_id: UserId::from_uuid(codec::uuid(&value["recipient"])?),
        occurrence_id: AlertOccurrenceId::from_uuid(codec::uuid(&value["occurrence"])?),
        subject: codec::read_subject(&value["subject"])?,
        kind: codec::read_kind(&value["kind"])?,
        origin: codec::read_origin(&value["origin"])?,
        trigger_at: codec::time(&value["trigger"])?,
        created_at: codec::time(&value["created"])?,
        subject_title: codec::text(&value["subject_title"])?,
        case_title: codec::text(&value["case_title"])?,
        case_reference: codec::text(&value["case_reference"])?,
        read_at: None,
        state: AlertState::Active,
        email: match codec::text(&value["email"])?.as_str() {
            "disabled" => AlertEmailStatus::Disabled,
            "pending" => AlertEmailStatus::Pending,
            _ => return Err(stored("invalid initial email state")),
        },
    };
    if encode(&result) != value {
        return Err(stored("alert record payload differs"));
    }
    let (kind, id) = codec::subject_key(result.subject);
    if row.try_get::<_, Uuid>("id").map_err(stored)? != result.id.as_uuid()
        || row.try_get::<_, Uuid>("recipient").map_err(stored)? != result.recipient_id.as_uuid()
        || row.try_get::<_, i16>("kind").map_err(stored)? != kind
        || row.try_get::<_, Uuid>("subject_id").map_err(stored)? != id
        || row.try_get::<_, Uuid>("case_id").map_err(stored)? != result.subject.case_id().as_uuid()
        || codec::row_time(row, "created_seconds", "created_nanos")? != Some(result.created_at)
    {
        return Err(stored("alert index projection differs"));
    }
    result.read_at = codec::row_time(row, "read_seconds", "read_nanos")?;
    let reason: Option<String> = row.try_get("resolved_reason").map_err(stored)?;
    match (
        codec::row_time(row, "resolved_seconds", "resolved_nanos")?,
        reason,
    ) {
        (None, None) => {}
        (Some(at), Some(reason)) => {
            result.state = AlertState::Resolved {
                at,
                reason: read_reason(&reason)?,
            }
        }
        _ => return Err(stored("partial alert resolution")),
    }
    Ok(result)
}
pub(super) fn reason_name(reason: AlertResolutionReason) -> &'static str {
    match reason {
        AlertResolutionReason::Superseded => "superseded",
        AlertResolutionReason::AttentionRecorded => "attention_recorded",
        AlertResolutionReason::TargetRetired => "target_retired",
        AlertResolutionReason::CancelledHearing => "cancelled_hearing",
        AlertResolutionReason::NoLongerEligible => "no_longer_eligible",
    }
}
fn read_reason(value: &str) -> Result<AlertResolutionReason, ApplicationError> {
    match value {
        "superseded" => Ok(AlertResolutionReason::Superseded),
        "attention_recorded" => Ok(AlertResolutionReason::AttentionRecorded),
        "target_retired" => Ok(AlertResolutionReason::TargetRetired),
        "cancelled_hearing" => Ok(AlertResolutionReason::CancelledHearing),
        "no_longer_eligible" => Ok(AlertResolutionReason::NoLongerEligible),
        _ => Err(stored("invalid alert resolution")),
    }
}
pub(super) fn resolve(
    tx: &mut Transaction<'_>,
    record: &mut AlertRecord,
    reason: AlertResolutionReason,
    now: OffsetDateTime,
) -> Result<(), ApplicationError> {
    if record.state == AlertState::Active {
        tx.execute("UPDATE alert_notifications SET resolved_seconds=$2,resolved_nanos=$3,resolved_reason=$4 WHERE id=$1 AND resolved_seconds IS NULL",
            &[&record.id.as_uuid(),&now.unix_timestamp(),&(now.nanosecond() as i32),&reason_name(reason)]).map_err(port)?;
        record.state = AlertState::Resolved { at: now, reason };
    }
    Ok(())
}
pub(super) fn current(
    tx: &mut Transaction<'_>,
    record: &mut AlertRecord,
    hasher: &dyn DocumentHasher,
    now: OffsetDateTime,
) -> Result<subject::Verified, ApplicationError> {
    subject::verify_origin(tx, record, hasher)?;
    let current = subject::load(tx, record.subject, hasher, now)?;
    let reason = episode_ended_after(tx, record, hasher)?
        .or_else(|| subject::reason(record.kind, &current, now));
    if let Some(reason) = reason {
        resolve(tx, record, reason, now)?;
    }
    Ok(current)
}

fn episode_ended_after(
    tx: &mut Transaction<'_>,
    record: &AlertRecord,
    hasher: &dyn DocumentHasher,
) -> Result<Option<AlertResolutionReason>, ApplicationError> {
    let AlertSubject::Deadline { case_id, id } = record.subject else {
        return Ok(None);
    };
    let review = record.kind == AlertKind::ReviewRequired;
    if !review && !matches!(record.kind, AlertKind::OverdueUnattended { .. }) {
        return Ok(None);
    }
    let row = tx
        .query_opt(
            "SELECT revision FROM case_deadline_revisions
        WHERE deadline_id=$1 AND case_id=$2 AND revision>$3
        AND (($4 AND action='correct') OR (NOT $4 AND attention->>'status'='recorded'))
        ORDER BY revision LIMIT 1",
            &[
                &id.as_uuid(),
                &case_id.as_uuid(),
                &i64::from(record.origin.revision),
                &review,
            ],
        )
        .map_err(port)?;
    let Some(row) = row else { return Ok(None) };
    let revision = application::deadlines::DeadlineRevision::new(
        u32::try_from(row.try_get::<_, i64>("revision").map_err(stored)?).map_err(stored)?,
    )
    .map_err(stored)?;
    let detail =
        crate::deadline_postgres::storage::detail(tx, case_id, id, Some(revision), hasher)?;
    let reason = if review {
        if detail.review_state() != application::deadline_tracking::DeadlineReviewState::Accepted {
            return Err(stored("alert review acceptance differs from its source"));
        }
        AlertResolutionReason::NoLongerEligible
    } else {
        if !matches!(
            detail.attention,
            application::deadlines::DeadlineAttention::Recorded { .. }
        ) {
            return Err(stored("alert attention transition differs from its source"));
        }
        AlertResolutionReason::AttentionRecorded
    };
    Ok(Some(reason))
}

pub(super) fn load(
    tx: &mut Transaction<'_>,
    id: AlertId,
    hasher: &dyn DocumentHasher,
) -> Result<AlertRecord, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT * FROM alert_notifications WHERE id=$1",
            &[&id.as_uuid()],
        )
        .map_err(port)?
        .ok_or(AlertError::NotFound)?;
    decode(&row, hasher)
}
