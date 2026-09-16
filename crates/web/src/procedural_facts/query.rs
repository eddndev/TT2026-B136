use super::request::parse_uuid;
use crate::error::ApiError;
use application::procedural_facts::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageQuery {
    limit: Option<u32>,
    after_id: Option<String>,
    status: Option<String>,
}
impl PageQuery {
    fn status(&self) -> Result<FactStatusFilter, ApiError> {
        match self.status.as_deref() {
            None | Some("all") => Ok(FactStatusFilter::All),
            Some("recorded") => Ok(FactStatusFilter::Recorded),
            Some("withdrawn") => Ok(FactStatusFilter::Withdrawn),
            _ => Err(invalid()),
        }
    }
    pub fn resolution(self) -> Result<ResolutionQuery, ApiError> {
        ResolutionQuery::new(
            self.limit.unwrap_or(20),
            self.after_id
                .as_deref()
                .map(|s| parse_uuid(s, "invalid_resolution_id").map(ResolutionId::from_uuid))
                .transpose()?,
            self.status()?,
        )
        .map_err(|_| invalid())
    }
    pub fn notification(self) -> Result<NotificationQuery, ApiError> {
        NotificationQuery::new(
            self.limit.unwrap_or(20),
            self.after_id
                .as_deref()
                .map(|s| parse_uuid(s, "invalid_notification_id").map(NotificationId::from_uuid))
                .transpose()?,
            self.status()?,
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
    pub fn validate(self) -> Result<FactHistoryQuery, ApiError> {
        FactHistoryQuery::new(self.limit.unwrap_or(10), self.before_revision).map_err(|_| invalid())
    }
}
pub(super) fn require_empty(query: Option<&str>) -> Result<(), ApiError> {
    if query.is_some_and(|value| !value.is_empty()) {
        return Err(invalid());
    }
    Ok(())
}
pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_query", "invalid procedural fact query parameters")
}
