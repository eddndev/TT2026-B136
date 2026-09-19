use super::cursor;
use crate::error::ApiError;
use application::{
    agenda::{AgendaKind, AgendaQuery},
    hearings::HearingStatusFilter,
};
use serde::Deserialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageQuery {
    from: String,
    until: String,
    limit: Option<u32>,
    kind: Option<String>,
    hearing_status: Option<String>,
    cursor: Option<String>,
}
impl PageQuery {
    pub fn validate(self) -> Result<AgendaQuery, ApiError> {
        let kind = match self.kind.as_deref().unwrap_or("all") {
            "all" => AgendaKind::All,
            "hearing" => AgendaKind::Hearing,
            "deadline" => AgendaKind::Deadline,
            _ => return Err(invalid()),
        };
        let status = match self.hearing_status.as_deref().unwrap_or("scheduled") {
            "scheduled" => HearingStatusFilter::Scheduled,
            "cancelled" => HearingStatusFilter::Cancelled,
            "all" => HearingStatusFilter::All,
            _ => return Err(invalid()),
        };
        let base = AgendaQuery::new(
            self.limit.unwrap_or(20),
            instant(&self.from)?,
            instant(&self.until)?,
            kind,
            status,
            None,
        )
        .map_err(|_| invalid())?;
        let after = self
            .cursor
            .map(|value| cursor::decode(&value, base))
            .transpose()?;
        AgendaQuery::new(base.limit(), base.from(), base.until(), kind, status, after)
            .map_err(|_| invalid())
    }
}
fn instant(value: &str) -> Result<OffsetDateTime, ApiError> {
    if value.len() != 20 || !value.is_ascii() || !value.ends_with('Z') {
        return Err(invalid());
    }
    let at = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| invalid())?;
    if at.format(&Rfc3339).map_err(|_| invalid())? != value {
        return Err(invalid());
    }
    Ok(at)
}
pub(super) fn kind(value: AgendaKind) -> &'static str {
    match value {
        AgendaKind::All => "all",
        AgendaKind::Hearing => "hearing",
        AgendaKind::Deadline => "deadline",
    }
}
pub(super) fn status(value: HearingStatusFilter) -> &'static str {
    match value {
        HearingStatusFilter::All => "all",
        HearingStatusFilter::Scheduled => "scheduled",
        HearingStatusFilter::Cancelled => "cancelled",
    }
}
pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_query", "invalid agenda query parameters")
}
