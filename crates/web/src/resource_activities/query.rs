use super::request::{number, revision, uuid};
use crate::error::ApiError;
use application::resource_activities::*;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    limit: Option<String>,
    after_id: Option<String>,
    kind: Option<String>,
    status: Option<String>,
}
impl Page {
    pub(super) fn validate(self) -> Result<ResourceActivityQuery, ApiError> {
        let kind = match self.kind.as_deref() {
            None => None,
            Some("hearing") => Some(ResourceActivityKind::Hearing),
            Some("deadline") => Some(ResourceActivityKind::Deadline),
            _ => return Err(invalid()),
        };
        let status = match self.status.as_deref() {
            None => None,
            Some("linked") => Some(ResourceActivityStatus::Linked),
            Some("unlinked") => Some(ResourceActivityStatus::Unlinked),
            _ => return Err(invalid()),
        };
        ResourceActivityQuery::new(
            self.limit.map(|v| number(&v)).transpose()?.unwrap_or(20),
            self.after_id
                .map(|v| uuid(&v).map(ResourceActivityId::from_uuid))
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
    limit: Option<String>,
    before_revision: Option<String>,
}
impl History {
    pub(super) fn validate(self) -> Result<ResourceActivityHistoryQuery, ApiError> {
        ResourceActivityHistoryQuery::new(
            self.limit.map(|v| number(&v)).transpose()?.unwrap_or(20),
            self.before_revision
                .map(|v| number(&v).and_then(revision))
                .transpose()?,
        )
        .map_err(|_| invalid())
    }
}
pub(super) fn require_empty(query: Option<&str>) -> Result<(), ApiError> {
    if query.is_some_and(|v| !v.is_empty()) {
        Err(invalid())
    } else {
        Ok(())
    }
}
pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_query", "invalid resource activity query")
}
