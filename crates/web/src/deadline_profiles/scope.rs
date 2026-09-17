use super::request;
use crate::error::ApiError;
use application::deadline_profiles::*;
use domain::cases::CaseId;
use serde::Deserialize;
#[derive(Deserialize)]
pub(super) struct RoutePath {
    case_id: Option<String>,
    id: Option<String>,
    revision: Option<String>,
}
impl RoutePath {
    pub(super) fn collection(&self) -> Result<DeadlineProfileCollection, ApiError> {
        self.case_id
            .as_ref()
            .map(|id| {
                request::uuid(id, "invalid_case_id")
                    .map(|id| DeadlineProfileCollection::ForCase(CaseId::from_uuid(id)))
            })
            .unwrap_or(Ok(DeadlineProfileCollection::Global))
    }
    pub(super) fn id(&self) -> Result<DeadlineProfileId, ApiError> {
        self.id
            .as_ref()
            .ok_or_else(ApiError::internal)
            .and_then(|id| {
                request::uuid(id, "invalid_deadline_profile_id").map(DeadlineProfileId::from_uuid)
            })
    }
    pub(super) fn revision(&self) -> Result<DeadlineProfileRevision, ApiError> {
        let invalid = || {
            ApiError::invalid_body(
                "invalid_deadline_profile_revision",
                "revision must be a positive u32",
            )
        };
        let s = self.revision.as_deref().ok_or_else(invalid)?;
        let n = request::parse_u32(s).ok_or_else(invalid)?;
        DeadlineProfileRevision::new(n).map_err(|_| invalid())
    }
}
pub(super) fn accepts(
    collection: DeadlineProfileCollection,
    command: &DeadlineProfileCommand,
) -> bool {
    match &command.change {
        DeadlineProfileChange::Publish { definition }
        | DeadlineProfileChange::Replace { definition, .. } => {
            collection.permits_mutation(definition.scope())
        }
        DeadlineProfileChange::Retire { .. } => true,
    }
}
pub(super) fn mismatch() -> ApiError {
    ApiError::invalid_body(
        "deadline_profile_command_mismatch",
        "route, profile, collection and action must agree",
    )
}
