use super::*;
use crate::{
    case_stages::CurrentCaseStage,
    cases::CurrentCaseAdministration,
    documents::{DocumentRecord, StageSupportReadLimits},
    procedural_facts::ResolutionSourceMaterial,
    typed_participants::ParticipantDetail,
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceMaterial {
    pub case_id: CaseId,
    pub base: Option<ResourceDetail>,
    /// Exact latest revision of the selected act; independent of the resource head.
    pub act_base: Option<ResourceDetail>,
    pub administration: CurrentCaseAdministration,
    pub stage: CurrentCaseStage,
    /// Present only for registration or correction of resource values.
    pub resolution: Option<Box<ResolutionSourceMaterial>>,
    pub appellants: Vec<ParticipantDetail>,
    /// One complete batch of new direct versions, at most two. Exact admitted
    /// versions retained from the corrected resource/act are not readmitted.
    pub records: Vec<DocumentRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourcePreparation {
    Ready(Box<ResourceMaterial>),
    /// Reauthorize first; return only the exact original operation and command.
    Replay(Box<ResourceDetail>),
}
/// Every operation revalidates the active account and current case membership.
/// Owner manages all, assigned Litigator manages, assigned Paralegal reads, Client
/// is denied. Closed cases retain reads and prohibit all new mutations.
/// Commit the read audit before returning protected data, including replay data.
pub trait ProceduralResourceStore: Send + Sync {
    /// Select heads before filters; paginate exclusive UUID order without fan-out.
    fn list(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: ResourceQuery,
        at: OffsetDateTime,
    ) -> Result<ResourcePage, ApplicationError>;
    fn get(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: ResourceId,
        revision: Option<ResourceRevision>,
        at: OffsetDateTime,
    ) -> Result<ResourceDetail, ApplicationError>;
    fn history(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: ResourceId,
        query: ResourceHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceHistoryPage, ApplicationError>;
    /// Resolve every selected historical source in this case under one authorized
    /// read. Bound records before decoding; no reference silently becomes a head.
    /// No state or operation reservation is written. Existing operation IDs must
    /// return their original exact receipt or OperationConflict, even after closure.
    fn prepare(
        &self,
        actor: UserId,
        case_id: CaseId,
        command: &ResourceCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<ResourcePreparation, ApplicationError>;
    /// Under the shared audit lock reauthorize, check active case, root, expected
    /// resource/act heads, operation uniqueness and all exact prepared sources and
    /// complete encrypted records. A retained reference cannot change its capture.
    /// Recheck observed administration and stage, returning a conflict if changed.
    /// Only then capture Clock and atomically append revision, act, receipt, audit.
    /// Exact operation replay returns the original record; differing reuse conflicts.
    /// No command changes case stage, deadlines, legal effects or document versions.
    fn commit(
        &self,
        actor: UserId,
        case_id: CaseId,
        prepared: PreparedResourceChange,
    ) -> Result<ResourceDetail, ApplicationError>;
}
pub trait ProceduralResourceWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: ResourceQuery,
    ) -> Result<ResourcePage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: ResourceId,
        revision: Option<ResourceRevision>,
    ) -> Result<ResourceDetail, ApplicationError>;
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: ResourceId,
        query: ResourceHistoryQuery,
    ) -> Result<ResourceHistoryPage, ApplicationError>;
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: ResourceCommand,
    ) -> Result<ResourceDraft, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: ResourceCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<ResourceDetail, ApplicationError>;
}
