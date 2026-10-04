use super::{cursor, query};
use crate::error::ApiError;
use application::agenda::{AgendaItem, AgendaPage, AgendaQuery, ResourceHearingAgendaOverview};
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub(super) fn page(page: AgendaPage, query: AgendaQuery) -> Result<Value, ApiError> {
    page.validate(&query).map_err(|_| ApiError::internal())?;
    let mut items = Vec::with_capacity(page.items.len());
    for item in page.items {
        let at = instant(item.key().map_err(|_| ApiError::internal())?.at());
        items.push(match item {
            AgendaItem::Hearing(hearing) => json!({"kind":"hearing", "at":at, "hearing":crate::hearings::agenda_overview(hearing)?}),
            AgendaItem::Deadline { case, deadline } => json!({"kind":"deadline", "at":at, "case_title":case.title, "case_reference":case.reference, "case_status":case.status.as_str(), "deadline":crate::deadlines::agenda_overview(*deadline)?}),
            AgendaItem::ResourceHearing { case, hearing } => json!({
                "kind":"resource_hearing", "at":at,
                "case_title":case.title, "case_reference":case.reference,
                "case_status":case.status.as_str(),
                "resource_hearing":resource_hearing(*hearing)?,
            }),
        });
    }
    Ok(
        json!({"from":query.from().format(&Rfc3339).map_err(|_| ApiError::internal())?, "until":query.until().format(&Rfc3339).map_err(|_| ApiError::internal())?, "kind":query::kind(query.kind()), "hearing_status":query::status(query.hearing_status()), "checked_at":instant(page.checked_at), "items":items, "complete":page.complete, "next_cursor":page.next_after.map(|value|cursor::encode(value, query))}),
    )
}
fn resource_hearing(value: ResourceHearingAgendaOverview) -> Result<Value, ApiError> {
    let scheduled_at = value
        .scheduled_at
        .value()
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())?;
    Ok(json!({
        "case_id":value.case_id.to_string(), "resource_id":value.resource_id.to_string(),
        "id":value.id.to_string(), "revision":value.revision.get(),
        "kind":value.kind.as_str(), "scheduled_at":scheduled_at,
        "modality":value.modality.as_str(), "participant_count":value.participant_count,
        "association_id":value.association_id.to_string(),
        "capture_digest":value.capture_digest.to_hex(),
    }))
}
fn instant(at: OffsetDateTime) -> Value {
    json!({"unix_seconds":at.unix_timestamp(), "nanosecond":at.nanosecond(), "offset_seconds":at.offset().whole_seconds()})
}
