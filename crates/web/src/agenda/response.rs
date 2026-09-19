use super::{cursor, query};
use crate::error::ApiError;
use application::agenda::{AgendaItem, AgendaPage, AgendaQuery};
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
        });
    }
    Ok(
        json!({"from":query.from().format(&Rfc3339).map_err(|_| ApiError::internal())?, "until":query.until().format(&Rfc3339).map_err(|_| ApiError::internal())?, "kind":query::kind(query.kind()), "hearing_status":query::status(query.hearing_status()), "checked_at":instant(page.checked_at), "items":items, "complete":page.complete, "next_cursor":page.next_after.map(|value|cursor::encode(value, query))}),
    )
}
fn instant(at: OffsetDateTime) -> Value {
    json!({"unix_seconds":at.unix_timestamp(), "nanosecond":at.nanosecond(), "offset_seconds":at.offset().whole_seconds()})
}
