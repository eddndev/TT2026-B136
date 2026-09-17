use super::request::parse_u32;
use crate::error::ApiError;
use application::deadline_profiles::*;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageQuery {
    limit: Option<String>,
    after_id: Option<String>,
    status: Option<String>,
}
impl PageQuery {
    pub fn validate(self) -> Result<DeadlineProfileQuery, ApiError> {
        let status = match self.status.as_deref() {
            None | Some("published") => DeadlineProfileStatusFilter::Published,
            Some("all") => DeadlineProfileStatusFilter::All,
            Some("retired") => DeadlineProfileStatusFilter::Retired,
            _ => return Err(invalid()),
        };
        DeadlineProfileQuery::new(
            number(self.limit.as_deref(), 20)?,
            self.after_id
                .map(|s| {
                    super::request::uuid(&s, "invalid_deadline_profile_id")
                        .map(DeadlineProfileId::from_uuid)
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
    limit: Option<String>,
    before_revision: Option<String>,
}
impl HistoryQuery {
    pub fn validate(self) -> Result<DeadlineProfileHistoryQuery, ApiError> {
        DeadlineProfileHistoryQuery::new(
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
    ApiError::invalid_body("invalid_query", "invalid deadline profile query parameters")
}
