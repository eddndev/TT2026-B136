use super::request::uuid;
use crate::error::ApiError;
use application::procedural_resources::*;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    limit: Option<u32>,
    after_id: Option<String>,
    kind: Option<String>,
    status: Option<String>,
}
impl Page {
    pub(super) fn validate(self) -> Result<ResourceQuery, ApiError> {
        let kind = match self.kind.as_deref() {
            None | Some("all") => None,
            Some(v) => Some(v.parse().map_err(|_| invalid())?),
        };
        let status = match self.status.as_deref() {
            None | Some("all") => None,
            Some(v) => Some(v.parse().map_err(|_| invalid())?),
        };
        ResourceQuery::new(
            self.limit.unwrap_or(20),
            self.after_id
                .map(|v| uuid(&v).map(ResourceId::from_uuid))
                .transpose()?,
            kind,
            status,
        )
        .map_err(|_| invalid())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct History {
    limit: Option<u32>,
    before_revision: Option<u32>,
}
impl History {
    pub(super) fn validate(self) -> Result<ResourceHistoryQuery, ApiError> {
        ResourceHistoryQuery::new(self.limit.unwrap_or(10), self.before_revision)
            .map_err(|_| invalid())
    }
}
pub(super) fn require_empty(q: Option<&str>) -> Result<(), ApiError> {
    if q.is_some_and(|v| !v.is_empty()) {
        Err(invalid())
    } else {
        Ok(())
    }
}
pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_query", "invalid procedural resource query")
}
