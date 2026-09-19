use super::*;
use crate::ApplicationError;
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};
/// Authenticate active account and membership before every lookup. Owner manages
/// all, assigned Litigator manages, assigned Paralegal reads, Client is denied.
/// Commit read audit before returning protected rows. Current target heads and
/// exact historical captures must come from the same authorized transaction.
pub trait ResourceActivityStore: Send + Sync {
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        query: ResourceActivityQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityPage, ApplicationError>;
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        revision: Option<ResourceActivityRevision>,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityView, ApplicationError>;
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        query: ResourceActivityHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityHistoryPage, ApplicationError>;
    /// Resolve historical sources exactly; never substitute current heads. Check
    /// operation replay before closure, but only after current authorization.
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceActivityCommand,
    ) -> Result<ResourceActivityPreparation, ApplicationError>;
    /// Reauthorize and compare expected association/resource heads, administration
    /// and complete source captures under the audit lock. Append revision and
    /// audit atomically; no endpoint mutation belongs here. Capture Clock only
    /// after checks. Authorized exact replay returns its original receipt.
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceActivityChange,
    ) -> Result<ResourceActivityDetail, ApplicationError>;
}
pub trait ResourceActivityWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        query: ResourceActivityQuery,
    ) -> Result<ResourceActivityPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        revision: Option<ResourceActivityRevision>,
    ) -> Result<ResourceActivityView, ApplicationError>;
    fn history(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        query: ResourceActivityHistoryQuery,
    ) -> Result<ResourceActivityHistoryPage, ApplicationError>;
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceActivityCommand,
    ) -> Result<ResourceActivityDraft, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceActivityCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<ResourceActivityDetail, ApplicationError>;
}
