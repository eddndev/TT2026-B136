use super::request::parse_uuid;
use crate::error::ApiError;
use application::hearing_results::*;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageQuery {
    limit: Option<u32>,
    after_id: Option<String>,
    status: Option<String>,
}
impl PageQuery {
    pub fn validate(self) -> Result<HearingResultQuery, ApiError> {
        let status = match self.status.as_deref() {
            None | Some("all") => HearingResultStatusFilter::All,
            Some("recorded") => HearingResultStatusFilter::Recorded,
            Some("withdrawn") => HearingResultStatusFilter::Withdrawn,
            _ => return Err(invalid()),
        };
        HearingResultQuery::new(
            self.limit.unwrap_or(20),
            self.after_id
                .map(|s| {
                    parse_uuid(&s, "invalid_hearing_result_id").map(HearingResultId::from_uuid)
                })
                .transpose()?,
            status,
        )
        .map_err(|_| invalid())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HistoryQuery {
    limit: Option<u32>,
    before_revision: Option<u32>,
}
impl HistoryQuery {
    pub fn validate(self) -> Result<HearingResultHistoryQuery, ApiError> {
        HearingResultHistoryQuery::new(self.limit.unwrap_or(10), self.before_revision)
            .map_err(|_| invalid())
    }
}
pub(super) fn require_empty(query: Option<&str>) -> Result<(), ApiError> {
    if query.is_some_and(|value| !value.is_empty()) {
        return Err(invalid());
    }
    Ok(())
}
pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_query", "invalid hearing result query parameters")
}
