use super::{ResourceHearingCreation, ResourceHearingPage, ResourceHearingReadQuery};
use crate::ApplicationError;
use domain::{
    cases::CaseId,
    identity::UserId,
    procedural_resources::ResourceId,
    resource_hearings::{ResourceHearingId, ResourceHearingRevision},
};

/// Historical reads reauthorize the active account and current case membership
/// under the shared audit lock before looking up resource or hearing identities.
/// Closed cases and archived resources remain readable. Reconstruct exact sources,
/// initial association and durable origin; commit the read audit before returning.
pub trait ResourceHearingReadStore: Send + Sync {
    /// Ascending UUIDs strictly after the cursor; include every created hearing
    /// regardless of later unlinking. Probe at most limit + 1 roots and return
    /// at most limit complete creations. More results require a full page and
    /// the last returned ID as continuation; otherwise continuation is absent.
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        query: ResourceHearingReadQuery,
    ) -> Result<ResourceHearingPage, ApplicationError>;

    /// None selects the stored head; Some requires that exact revision without
    /// substitution. An authorized absence returns ResourceActivityError::NotFound.
    /// A corrupt or incomplete origin is an integrity error, never an absence.
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        hearing: ResourceHearingId,
        revision: Option<ResourceHearingRevision>,
    ) -> Result<ResourceHearingCreation, ApplicationError>;
}

pub trait ResourceHearingReadWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        query: ResourceHearingReadQuery,
    ) -> Result<ResourceHearingPage, ApplicationError>;

    fn get(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        hearing: ResourceHearingId,
        revision: Option<ResourceHearingRevision>,
    ) -> Result<ResourceHearingCreation, ApplicationError>;
}
