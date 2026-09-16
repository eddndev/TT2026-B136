use super::request::parse_u32;
use crate::error::ApiError;
use application::judicial_calendars::*;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageQuery {
    limit: Option<String>,
    after_id: Option<String>,
    status: Option<String>,
    jurisdiction: Option<String>,
    entity_code: Option<String>,
}
impl PageQuery {
    pub fn validate(self) -> Result<JudicialCalendarQuery, ApiError> {
        let status = match self.status.as_deref() {
            None | Some("published") => JudicialCalendarStatusFilter::Published,
            Some("all") => JudicialCalendarStatusFilter::All,
            Some("retired") => JudicialCalendarStatusFilter::Retired,
            _ => return Err(invalid()),
        };
        JudicialCalendarQuery::new(
            number(self.limit.as_deref(), 20)?,
            self.after_id.map(|s| super::parse_id(&s)).transpose()?,
            status,
            self.jurisdiction
                .map(|j| j.parse().map_err(|_| invalid()))
                .transpose()?,
            self.entity_code.as_deref(),
        )
        .map_err(|_| invalid())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HistoryQuery {
    limit: Option<String>,
    before_revision: Option<String>,
}
impl HistoryQuery {
    pub fn validate(self) -> Result<JudicialCalendarHistoryQuery, ApiError> {
        JudicialCalendarHistoryQuery::new(
            number(self.limit.as_deref(), 10)?,
            self.before_revision
                .map(|s| parse_u32(&s).ok_or_else(invalid))
                .transpose()?,
        )
        .map_err(|_| invalid())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DaysQuery {
    from: String,
    through: String,
}
impl DaysQuery {
    pub fn validate(self) -> Result<JudicialCalendarDaysQuery, ApiError> {
        JudicialCalendarDaysQuery::new(
            self.from.parse().map_err(|_| invalid())?,
            self.through.parse().map_err(|_| invalid())?,
        )
        .map_err(|_| invalid())
    }
}
fn number(value: Option<&str>, default: u32) -> Result<u32, ApiError> {
    value
        .map(|v| parse_u32(v).ok_or_else(invalid))
        .unwrap_or(Ok(default))
}
pub(super) fn require_empty(query: Option<&str>) -> Result<(), ApiError> {
    if query.is_some_and(|q| !q.is_empty()) {
        return Err(invalid());
    }
    Ok(())
}
pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body(
        "invalid_query",
        "invalid judicial calendar query parameters",
    )
}
