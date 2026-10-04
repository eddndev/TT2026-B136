use super::unused::Unused;
use application::{resource_hearings::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    procedural_resources::ResourceId,
    resource_hearings::{ResourceHearingId, ResourceHearingRevision},
};
impl ResourceHearingWorkflow for Unused {
    fn prepare(
        &self,
        _: &str,
        _: CaseId,
        _: ResourceId,
        _: ResourceHearingCommand,
    ) -> Result<ResourceHearingDraft, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
    fn submit(
        &self,
        _: &str,
        _: CaseId,
        _: ResourceId,
        _: ResourceHearingCommand,
        _: Sha256Digest,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
}
impl ResourceHearingReadWorkflow for Unused {
    fn list(
        &self,
        _: &str,
        _: CaseId,
        _: ResourceId,
        _: ResourceHearingReadQuery,
    ) -> Result<ResourceHearingPage, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
    fn get(
        &self,
        _: &str,
        _: CaseId,
        _: ResourceId,
        _: ResourceHearingId,
        _: Option<ResourceHearingRevision>,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
}
