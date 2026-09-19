use super::stored;
use application::{alerts::*, deadlines::DeadlineId, hearings::HearingId, ApplicationError};
use domain::{
    alerts::AlertLeadHours,
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
};
use postgres::Row;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) fn payload(
    value: &Value,
    hasher: &dyn DocumentHasher,
) -> Result<(Vec<u8>, Vec<u8>), ApplicationError> {
    let bytes = serde_json::to_vec(value).map_err(stored)?;
    if bytes.len() > 32768 {
        return Err(stored("alert payload exceeds bounds"));
    }
    let digest = hasher.hash_bytes(&bytes).as_bytes().to_vec();
    Ok((bytes, digest))
}
pub(super) fn read(row: &Row, hasher: &dyn DocumentHasher) -> Result<Value, ApplicationError> {
    let bytes: Vec<u8> = row.try_get("payload").map_err(stored)?;
    let digest: Vec<u8> = row.try_get("payload_digest").map_err(stored)?;
    if bytes.len() > 32768 || hasher.hash_bytes(&bytes).as_bytes().as_slice() != digest {
        return Err(stored("alert payload digest differs"));
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(stored)?;
    if serde_json::to_vec(&value).map_err(stored)? != bytes {
        return Err(stored("alert payload is not canonical"));
    }
    Ok(value)
}
pub(super) fn text(value: &Value) -> Result<String, ApplicationError> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| stored("expected alert string"))
}
pub(super) fn integer(value: &Value) -> Result<i64, ApplicationError> {
    value
        .as_i64()
        .ok_or_else(|| stored("expected alert integer"))
}
pub(super) fn boolean(value: &Value) -> Result<bool, ApplicationError> {
    value
        .as_bool()
        .ok_or_else(|| stored("expected alert boolean"))
}
pub(super) fn uuid(value: &Value) -> Result<Uuid, ApplicationError> {
    let text = text(value)?;
    let id = Uuid::parse_str(&text).map_err(stored)?;
    if id.to_string() != text {
        return Err(stored("noncanonical alert UUID"));
    }
    Ok(id)
}
pub(super) fn digest(value: &Value) -> Result<Sha256Digest, ApplicationError> {
    let text = text(value)?;
    let digest = Sha256Digest::from_hex(&text).map_err(stored)?;
    if digest.to_hex() != text {
        return Err(stored("noncanonical alert digest"));
    }
    Ok(digest)
}
pub(super) fn instant(at: OffsetDateTime) -> Value {
    json!([at.unix_timestamp(), at.nanosecond()])
}
pub(super) fn optional(at: Option<OffsetDateTime>) -> Value {
    at.map(instant).unwrap_or(Value::Null)
}
pub(super) fn time(value: &Value) -> Result<OffsetDateTime, ApplicationError> {
    let parts = value
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or_else(|| stored("invalid alert instant"))?;
    let seconds = integer(&parts[0])?;
    let nanos = integer(&parts[1])?;
    if !(0..1_000_000_000).contains(&nanos) {
        return Err(stored("invalid alert nanoseconds"));
    }
    let at = OffsetDateTime::from_unix_timestamp_nanos(
        i128::from(seconds) * 1_000_000_000 + i128::from(nanos),
    )
    .map_err(stored)?;
    if !(1..=9999).contains(&at.year()) {
        return Err(stored("unsupported alert instant"));
    }
    Ok(at)
}
pub(super) fn optional_time(value: &Value) -> Result<Option<OffsetDateTime>, ApplicationError> {
    if value.is_null() {
        Ok(None)
    } else {
        time(value).map(Some)
    }
}
pub(super) fn row_time(
    row: &Row,
    seconds: &str,
    nanos: &str,
) -> Result<Option<OffsetDateTime>, ApplicationError> {
    let sec: Option<i64> = row.try_get(seconds).map_err(stored)?;
    let nano: Option<i32> = row.try_get(nanos).map_err(stored)?;
    match (sec, nano) {
        (None, None) => Ok(None),
        (Some(s), Some(n)) => time(&json!([s, n])).map(Some),
        _ => Err(stored("partial alert instant")),
    }
}
pub(super) fn subject(value: AlertSubject) -> Value {
    let (kind, id) = subject_key(value);
    json!([kind, value.case_id().as_uuid(), id])
}
pub(super) fn subject_key(value: AlertSubject) -> (i16, Uuid) {
    match value {
        AlertSubject::Hearing { id, .. } => (0, id.as_uuid()),
        AlertSubject::Deadline { id, .. } => (1, id.as_uuid()),
    }
}
pub(super) fn read_subject(value: &Value) -> Result<AlertSubject, ApplicationError> {
    let case_id = CaseId::from_uuid(uuid(&value[1])?);
    let id = uuid(&value[2])?;
    match integer(&value[0])? {
        0 => Ok(AlertSubject::Hearing {
            case_id,
            id: HearingId::from_uuid(id),
        }),
        1 => Ok(AlertSubject::Deadline {
            case_id,
            id: DeadlineId::from_uuid(id),
        }),
        _ => Err(stored("invalid alert family")),
    }
}
pub(super) fn kind(value: AlertKind) -> Value {
    match value {
        AlertKind::Upcoming {
            lead_hours,
            activity_at,
        } => json!(["upcoming", lead_hours.get(), instant(activity_at)]),
        AlertKind::OverdueUnattended { due_at } => json!(["overdue", instant(due_at)]),
        AlertKind::ReviewRequired => json!(["review"]),
        AlertKind::DueChangedSoon {
            previous_due_at,
            current_due_at,
        } => json!(["changed", instant(previous_due_at), instant(current_due_at)]),
    }
}
pub(super) fn read_kind(value: &Value) -> Result<AlertKind, ApplicationError> {
    match text(&value[0])?.as_str() {
        "upcoming" => Ok(AlertKind::Upcoming {
            lead_hours: AlertLeadHours::new(u16::try_from(integer(&value[1])?).map_err(stored)?)
                .map_err(stored)?,
            activity_at: time(&value[2])?,
        }),
        "overdue" => Ok(AlertKind::OverdueUnattended {
            due_at: time(&value[1])?,
        }),
        "review" => Ok(AlertKind::ReviewRequired),
        "changed" => Ok(AlertKind::DueChangedSoon {
            previous_due_at: time(&value[1])?,
            current_due_at: time(&value[2])?,
        }),
        _ => Err(stored("invalid alert kind")),
    }
}
pub(super) fn origin(value: AlertOrigin) -> Value {
    json!([value.revision, value.evidence_digest.to_hex()])
}
pub(super) fn read_origin(value: &Value) -> Result<AlertOrigin, ApplicationError> {
    let revision = u32::try_from(integer(&value[0])?).map_err(stored)?;
    if revision == 0 {
        return Err(stored("zero alert origin revision"));
    }
    Ok(AlertOrigin {
        revision,
        evidence_digest: digest(&value[1])?,
    })
}
