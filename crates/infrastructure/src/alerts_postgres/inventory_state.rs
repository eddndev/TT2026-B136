use super::super::{codec, invalidate, port, stored};
use application::{
    alerts::*,
    deadlines::{DeadlineAttention, DeadlineRevision},
    hearings::{HearingRevision, HearingStatus},
    ApplicationError,
};
use domain::{crypto::DocumentHasher, identity::UserId};
use postgres::{Row, Transaction};
use serde_json::{json, Value};
use uuid::Uuid;

pub(super) fn root(
    tx: &mut Transaction<'_>,
    kind: i16,
    id: Uuid,
    case: Uuid,
) -> Result<AlertSubject, ApplicationError> {
    let table = match kind {
        0 => "case_hearings",
        1 => "case_deadlines",
        _ => return Err(stored("invalid alert subject family")),
    };
    let row = tx
        .query_opt(&format!("SELECT case_id FROM {table} WHERE id=$1"), &[&id])
        .map_err(port)?
        .ok_or_else(|| stored("alert subject root absent"))?;
    if row.try_get::<_, Uuid>("case_id").map_err(stored)? != case {
        return Err(stored("alert subject belongs to another case"));
    }
    codec::read_subject(&json!([kind, case, id]))
}
pub(super) fn validate(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let kind: i16 = row.try_get("kind").map_err(stored)?;
    let id: Uuid = row.try_get("id").map_err(stored)?;
    let case: Uuid = row.try_get("case_id").map_err(stored)?;
    let target = root(tx, kind, id, case)?;
    if row.try_get::<_, i64>("generation").map_err(stored)? < 1 {
        return Err(stored("invalid alert state generation"));
    }
    let value = codec::read(row, hasher)?;
    if value["snapshot"].is_null() {
        if value != invalidate::initial() || !row.try_get::<_, bool>("dirty").map_err(stored)? {
            return Err(stored("unobserved alert state differs"));
        }
        return Ok(());
    }
    let snapshot = &value["snapshot"];
    let origin = codec::read_origin(&snapshot["origin"])?;
    let due = codec::optional_time(&snapshot["due"])?;
    let review = codec::boolean(&snapshot["review"])?;
    let attention = codec::boolean(&snapshot["attention_pending"])?;
    let responsible = if snapshot["responsible"].is_null() {
        None
    } else {
        Some(UserId::from_uuid(codec::uuid(&snapshot["responsible"])?))
    };
    let expected = json!({"origin":codec::origin(origin),"due":codec::optional(due),"review":review,"attention_pending":attention,"responsible":responsible.map(|id|id.as_uuid())});
    if expected != *snapshot {
        return Err(stored("alert state snapshot differs"));
    }
    let (digest, captured_due, captured_responsible, captured_attention) = match target {
        AlertSubject::Hearing { case_id, id } => {
            let detail = crate::hearing_postgres::storage::detail(
                tx,
                case_id,
                id,
                Some(HearingRevision::new(origin.revision).map_err(stored)?),
                hasher,
            )?;
            let due = (detail.snapshot.status == HearingStatus::Scheduled)
                .then_some(detail.snapshot.values.scheduled_at().utc());
            if review || attention || responsible.is_some() {
                return Err(stored("hearing alert state has deadline attributes"));
            }
            if due != codec::optional_time(&snapshot["due"])? {
                return Err(stored("hearing state date differs"));
            }
            (detail.snapshot.receipt.submission_digest, due, None, false)
        }
        AlertSubject::Deadline { case_id, id } => {
            let detail = crate::deadline_postgres::storage::detail(
                tx,
                case_id,
                id,
                Some(DeadlineRevision::new(origin.revision).map_err(stored)?),
                hasher,
            )?;
            (
                detail.receipt.capture_digest,
                detail.operational_due_at(),
                Some(detail.definition.responsible),
                matches!(detail.attention, DeadlineAttention::Pending),
            )
        }
    };
    if digest != origin.evidence_digest
        || due.is_some() && due != captured_due
        || responsible != captured_responsible
        || attention != captured_attention
    {
        return Err(stored("alert state differs from captured origin"));
    }
    let last_due = codec::optional_time(&value["last_due"])?;
    if due.is_some() && last_due != due {
        return Err(stored("alert last due differs"));
    }
    let episode = |field: &str| -> Result<Option<Uuid>, ApplicationError> {
        if value[field].is_null() {
            Ok(None)
        } else {
            codec::uuid(&value[field]).map(Some)
        }
    };
    let review_episode = episode("review_episode")?;
    let overdue_episode = episode("overdue_episode")?;
    if review_episode.is_some() != review
        || overdue_episode.is_some() && (!attention || due.is_none())
    {
        return Err(stored("alert episode has no applicable state"));
    }
    let changed = if value["changed_episode"].is_null() {
        Value::Null
    } else {
        let values = &value["changed_episode"];
        let id = codec::uuid(&values[0])?;
        let before = codec::time(&values[1])?;
        let after = codec::time(&values[2])?;
        if before == after || last_due != Some(after) {
            return Err(stored("alert changed episode differs"));
        }
        json!([id, codec::instant(before), codec::instant(after)])
    };
    let expected = json!({"snapshot":snapshot,"last_due":codec::optional(last_due),"review_episode":review_episode,"overdue_episode":overdue_episode,"changed_episode":changed});
    if expected != value {
        return Err(stored("alert subject state is not canonical"));
    }
    Ok(())
}

pub(super) fn cursor(tx: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let rows = tx
        .query("SELECT * FROM alert_scan_cursor LIMIT 2", &[])
        .map_err(port)?;
    let [row] = rows.as_slice() else {
        return Err(stored("alert cursor must have one row"));
    };
    if !row.try_get::<_, bool>("singleton").map_err(stored)?
        || row.try_get::<_, i64>("cycle").map_err(stored)? < 0
    {
        return Err(stored("invalid alert scan counter"));
    }
    let kind: i16 = row.try_get("kind").map_err(stored)?;
    if !matches!(kind, 0 | 1) {
        return Err(stored("invalid scan family"));
    }
    let id: Option<Uuid> = row.try_get("id").map_err(stored)?;
    if let Some(id) = id {
        cursor_root(tx, kind, id)?;
    }
    let active_kind: Option<i16> = row.try_get("active_kind").map_err(stored)?;
    let active_id: Option<Uuid> = row.try_get("active_id").map_err(stored)?;
    let after: Option<Uuid> = row.try_get("after_recipient").map_err(stored)?;
    match (active_kind, active_id) {
        (None, None) if after.is_none() => (),
        (Some(kind), Some(id)) => {
            cursor_root(tx, kind, id)?;
            if tx
                .query_opt(
                    "SELECT id FROM alert_subject_state WHERE kind=$1 AND id=$2",
                    &[&kind, &id],
                )
                .map_err(port)?
                .is_none()
            {
                return Err(stored("active scan state absent"));
            }
            if let Some(user) = after {
                if tx
                    .query_opt("SELECT id FROM users WHERE id=$1", &[&user])
                    .map_err(port)?
                    .is_none()
                {
                    return Err(stored("scan recipient absent"));
                }
            }
        }
        _ => return Err(stored("partial alert scan cursor")),
    }
    codec::row_time(row, "next_seconds", "next_nanos")?;
    Ok(())
}
fn cursor_root(tx: &mut Transaction<'_>, kind: i16, id: Uuid) -> Result<(), ApplicationError> {
    let table = match kind {
        0 => "case_hearings",
        1 => "case_deadlines",
        _ => return Err(stored("invalid scan subject family")),
    };
    if tx
        .query_opt(&format!("SELECT id FROM {table} WHERE id=$1"), &[&id])
        .map_err(port)?
        .is_none()
    {
        return Err(stored("alert scan root absent"));
    }
    Ok(())
}
