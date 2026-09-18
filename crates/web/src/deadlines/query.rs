use super::request::parse_u32;
use crate::error::ApiError;
use application::deadlines::*;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageQuery {
    limit: Option<String>,
    after_id: Option<String>,
    status: Option<String>,
}
impl PageQuery {
    pub fn validate(self) -> Result<DeadlineQuery, ApiError> {
        let status = match self.status.as_deref() {
            None | Some("active") => DeadlineStatusFilter::Active,
            Some("all") => DeadlineStatusFilter::All,
            Some("retired") => DeadlineStatusFilter::Retired,
            _ => return Err(invalid()),
        };
        DeadlineQuery::new(
            number(self.limit.as_deref(), 20)?,
            self.after_id
                .map(|s| super::request::uuid(&s, "invalid_deadline_id").map(DeadlineId::from_uuid))
                .transpose()?,
            status,
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
    pub fn validate(self) -> Result<DeadlineHistoryQuery, ApiError> {
        DeadlineHistoryQuery::new(
            number(self.limit.as_deref(), 10)?,
            self.before_revision
                .map(|s| parse_u32(&s).ok_or_else(invalid))
                .transpose()?,
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
    ApiError::invalid_body("invalid_query", "invalid deadline query parameters")
}
