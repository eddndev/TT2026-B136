use super::*;
use crate::{resource_activities::ResourceId, ApplicationError};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId};
/// Authorize active membership before lookup. Read exact source captures under
/// the audit lock; reject ordinary operations as contextual replays.
pub trait ResourceDeadlineStore: Send + Sync {
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceDeadlineCommand,
    ) -> Result<ResourceDeadlinePreparation, ApplicationError>;
    /// Reauthorize, resolve and compare every reviewed head before writing both
    /// revisions and their audit events in one transaction. Exact replay requires
    /// both receipts and the committed contextual origin marker.
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceDeadline,
    ) -> Result<ResourceDeadlineResult, ApplicationError>;
}
pub trait ResourceDeadlineWorkflow: Send + Sync {
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceDeadlineCommand,
    ) -> Result<ResourceDeadlineDraft, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceDeadlineCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<ResourceDeadlineResult, ApplicationError>;
}
