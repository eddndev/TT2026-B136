use application::{audit_query::*, ApplicationError};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

pub(super) fn page(
    page: AuditEventPage,
    query: &AuditEventQuery,
) -> Result<Value, ApplicationError> {
    if page.events.len() > query.limit() as usize
        || page
            .snapshot_max_sequence
            .is_some_and(|value| value > i64::MAX as u64)
        || page.has_more != page.next_cursor.is_some()
        || page.next_cursor.as_ref().is_some_and(|value| {
            value.is_empty()
                || value.len() > 4096
                || !value.bytes().all(|byte| byte.is_ascii_graphic())
        })
    {
        return Err(invalid());
    }
    let mut text_bytes = 0usize;
    let mut events = Vec::with_capacity(page.events.len());
    for event in page.events {
        if event.sequence > i64::MAX as u64 {
            return Err(invalid());
        }
        for value in [&event.actor, &event.action, &event.resource] {
            text_bytes = text_bytes
                .checked_add(value.len())
                .filter(|value| *value <= MAX_AUDIT_PAGE_TEXT_BYTES)
                .ok_or(ApplicationError::AuditQueryCapacityExceeded)?;
        }
        events.push(json!({
            "sequence": event.sequence.to_string(),
            "timestamp": event.timestamp.format(&Rfc3339).map_err(|_| invalid())?,
            "actor": event.actor, "action": event.action, "resource": event.resource,
        }));
    }
    Ok(json!({
        "checked_at": page.checked_at.format(&Rfc3339).map_err(|_| invalid())?,
        "snapshot_max_sequence": page.snapshot_max_sequence.map(|value| value.to_string()),
        "events": events, "has_more": page.has_more, "next_cursor": page.next_cursor,
    }))
}
fn invalid() -> ApplicationError {
    ApplicationError::Port("audit response violates its bounds".into())
}
