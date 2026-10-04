use super::request;
use crate::error::ApiError;
use application::resource_hearings::ResourceHearingReadQuery;
use domain::resource_hearings::ResourceHearingId;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    limit: Option<String>,
    after_id: Option<String>,
}
impl Page {
    pub(super) fn validate(self) -> Result<ResourceHearingReadQuery, ApiError> {
        let limit = self
            .limit
            .map(|v| request::number(&v))
            .transpose()?
            .unwrap_or(10);
        ResourceHearingReadQuery::new(
            u16::try_from(limit).map_err(|_| request::invalid())?,
            self.after_id
                .map(|v| request::uuid(&v).map(ResourceHearingId::from_uuid))
                .transpose()?,
        )
        .map_err(|_| request::invalid())
    }
}
pub(super) fn empty(query: Option<&str>) -> Result<(), ApiError> {
    if query.is_some_and(|v| !v.is_empty()) {
        Err(request::invalid())
    } else {
        Ok(())
    }
}
