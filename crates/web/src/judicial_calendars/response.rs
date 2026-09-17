use super::projection as p;
use crate::error::ApiError;
use application::judicial_calendars::*;
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
pub(super) fn draft(
    v: JudicialCalendarDraft,
    expected: &JudicialCalendarCommand,
) -> Result<Value, ApiError> {
    if v.command != *expected
        || v.result_revision != expected.result_revision()?
        || v.initial_scope != *v.values.scope()
    {
        return Err(ApiError::internal());
    }
    if let JudicialCalendarChange::Publish { values }
    | JudicialCalendarChange::Replace { values, .. } = &expected.change
    {
        if *values != v.values {
            return Err(ApiError::internal());
        }
    }
    Ok(
        json!({"actor_id":v.actor,"command":p::command(&v.command),"result_revision":v.result_revision.get(),"values":p::values(&v.values),"values_digest":v.values_digest.to_hex(),"initial_scope":p::scope(&v.initial_scope),"submission_digest":v.submission_digest.to_hex()}),
    )
}
pub(super) fn detail(
    v: JudicialCalendarDetail,
    id: JudicialCalendarId,
    revision: Option<JudicialCalendarRevision>,
) -> Result<Value, ApiError> {
    if v.id != id || revision.is_some_and(|r| r != v.revision) {
        return Err(ApiError::internal());
    }
    let mut row = entry(JudicialCalendarHistoryEntry::from(&v), id)?;
    row["values"] = p::values(&v.values);
    Ok(row)
}
fn entry(v: JudicialCalendarHistoryEntry, id: JudicialCalendarId) -> Result<Value, ApiError> {
    let shape = match v.receipt.action {
        JudicialCalendarAction::Publish => {
            v.receipt.expected_revision == 0
                && v.reason.is_none()
                && v.status == JudicialCalendarStatus::Published
        }
        JudicialCalendarAction::Replace => {
            v.receipt.expected_revision > 0
                && v.reason.is_some()
                && v.status == JudicialCalendarStatus::Published
        }
        JudicialCalendarAction::Retire => {
            v.receipt.expected_revision > 0
                && v.reason.is_some()
                && v.status == JudicialCalendarStatus::Retired
        }
    };
    if v.id != id
        || !shape
        || v.receipt.expected_revision.checked_add(1) != Some(v.revision.get())
        || v.recorded_by.email.trim().is_empty()
    {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"id":v.id.to_string(),"revision":v.revision.get(),"status":v.status.as_str(),"values_digest":v.values_digest.to_hex(),"reason":v.reason.as_ref().map(|r|r.as_str()),"receipt":p::receipt(&v.receipt),"recorded_at":v.recorded_at.format(&Rfc3339).map_err(|_|ApiError::internal())?,"recorded_by":{"id":v.recorded_by.id,"email":v.recorded_by.email}}),
    )
}
pub(super) fn page(v: JudicialCalendarPage) -> Value {
    json!({"calendars":v.calendars.iter().map(|v|json!({"id":v.id.to_string(),"revision":v.revision.get(),"status":v.status.as_str(),"scope":p::scope(&v.scope),"coverage":p::coverage(v.coverage),"values_digest":v.values_digest.to_hex(),"has_unresolved":v.has_unresolved})).collect::<Vec<_>>(),"has_more":v.has_more,"next_after_id":v.next_after_id.map(|id|id.to_string())})
}
pub(super) fn history(
    v: JudicialCalendarHistoryPage,
    id: JudicialCalendarId,
) -> Result<Value, ApiError> {
    Ok(
        json!({"revisions":v.revisions.into_iter().map(|v|entry(v,id)).collect::<Result<Vec<_>,_>>()?,"has_more":v.has_more,"next_before_revision":v.next_before_revision.map(|r|r.get())}),
    )
}
pub(super) fn days(
    v: JudicialCalendarDays,
    id: JudicialCalendarId,
    revision: JudicialCalendarRevision,
    q: JudicialCalendarDaysQuery,
) -> Result<Value, ApiError> {
    let expected = q.through().days_since_epoch() - q.from().days_since_epoch() + 1;
    if v.calendar_id != id
        || v.revision != revision
        || v.days.len() != expected as usize
        || v.days
            .iter()
            .enumerate()
            .any(|(i, d)| d.date().days_since_epoch() != q.from().days_since_epoch() + i as i32)
    {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"calendar_id":v.calendar_id.to_string(),"revision":v.revision.get(),"values_digest":v.values_digest.to_hex(),"days":v.days.iter().map(p::day).collect::<Vec<_>>()}),
    )
}
