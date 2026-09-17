use super::*;
use crate::ApplicationError;
use domain::{clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};
/// Exact current base and immutable R1 scope; reading does not reserve an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfilePreparation {
    pub collection: DeadlineProfileCollection,
    pub profile_id: DeadlineProfileId,
    pub base: Option<DeadlineProfileDetail>,
    pub initial_scope: Option<DeadlineProfileScope>,
}
/// Resolve case membership even when a ForCase query only returns global profiles.
/// Reads allow closed cases and filter authorized scope before pagination. Exact
/// reads and summaries validate the persisted algorithm and immutable R1 scope.
pub trait DeadlineProfileStore: Send + Sync {
    fn list(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        query: DeadlineProfileQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfilePage, ApplicationError>;
    /// Resolve the requested exact revision and verify its scope against R1.
    fn get(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        revision: Option<DeadlineProfileRevision>,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfileDetail, ApplicationError>;
    fn history(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        query: DeadlineProfileHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfileHistoryPage, ApplicationError>;
    fn prepare(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        command: &DeadlineProfileCommand,
    ) -> Result<DeadlineProfilePreparation, ApplicationError>;
    /// Under the shared audit lock, revalidate active Owner, collection, private case
    /// access and active status, expected head, R1 scope, terminal status and operation.
    /// Read Clock there; revision, audit and source event commit or roll back together.
    /// Compare the complete prepared base to the immutable stored revision.
    fn commit(
        &self,
        actor: UserId,
        prepared: PreparedDeadlineProfileChange,
    ) -> Result<DeadlineProfileDetail, ApplicationError>;
}
pub trait DeadlineProfileWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        query: DeadlineProfileQuery,
    ) -> Result<DeadlineProfilePage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        revision: Option<DeadlineProfileRevision>,
    ) -> Result<DeadlineProfileDetail, ApplicationError>;
    fn history(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        query: DeadlineProfileHistoryQuery,
    ) -> Result<DeadlineProfileHistoryPage, ApplicationError>;
    fn prepare(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        command: DeadlineProfileCommand,
    ) -> Result<DeadlineProfileDraft, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        command: DeadlineProfileCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<DeadlineProfileDetail, ApplicationError>;
}
