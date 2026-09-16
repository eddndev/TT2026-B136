use super::*;
use crate::{
    cases::CurrentCaseAdministration,
    documents::{DocumentRecord, StageSupportReadLimits},
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};

/// Exact inputs only; preparing does not reserve a revision or operation identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactPreparation {
    pub case_id: CaseId,
    pub base: Option<FactDetail>,
    pub observed_administration: CurrentCaseAdministration,
    pub source_material: FactSourceMaterial,
    /// Direct versions deduplicated by document and version, never by function.
    pub records: Vec<DocumentRecord>,
}

/// Repository implementations must enforce current authorization on every operation.
/// Owner has access to all cases; assigned Litigator may read and write; assigned
/// Paralegal may read. Client is denied. Missing foreign sources must not disclose
/// their existence. Read queries retain access to closed cases when authorized.
pub trait ProceduralFactStore: Send + Sync {
    /// Select heads before applying status filters and exclusive UUID pagination.
    fn list_resolutions(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: ResolutionQuery,
        at: OffsetDateTime,
    ) -> Result<ResolutionPage, ApplicationError>;
    /// The parent is an existing resolution of this case, independent of its head status.
    fn list_notifications(
        &self,
        actor: UserId,
        case_id: CaseId,
        resolution_id: ResolutionId,
        query: NotificationQuery,
        at: OffsetDateTime,
    ) -> Result<NotificationPage, ApplicationError>;
    /// Exact sources and their captured states never resolve through their current heads.
    fn get(
        &self,
        actor: UserId,
        case_id: CaseId,
        target: FactTarget,
        revision: Option<FactRevision>,
        at: OffsetDateTime,
    ) -> Result<FactDetail, ApplicationError>;
    fn history(
        &self,
        actor: UserId,
        case_id: CaseId,
        target: FactTarget,
        query: FactHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<FactHistoryPage, ApplicationError>;
    /// Resolve each source in the case and verify agreement membership in its exact
    /// result. A resolution selects at most one result and one direct document. A
    /// notification selects one exact parent, four participant references, two result
    /// references and two direct documents at most. Preserve function-specific
    /// locators; deduplication must not merge different revisions or versions.
    ///
    /// Return exact material for value and receipt verification, not caller-supplied
    /// digests or labels. Repeated result references share one material revision even
    /// when they select different agreements. Derive every readable source view
    /// from validated material before returning a draft or historical detail.
    ///
    /// Historical sources retain their digests and captured admission. They do not
    /// recursively add records to the direct batch. Withdrawal copies base sources
    /// and returns no records. Bound records before decoding and return the entire
    /// direct batch to the service for one admission pass; never split it into lots.
    fn prepare(
        &self,
        actor: UserId,
        case_id: CaseId,
        command: &ProceduralFactCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<FactPreparation, ApplicationError>;
    /// Under the common audit lock, recheck actor, membership, active case, stable
    /// root, expected head, operation uniqueness and every prepared exact source.
    /// Recheck complete direct records against the admitted batch before writing.
    /// Capture current administration and Clock after the lock; observations made
    /// during preparation are not administrative CAS tokens. An unrevised baseline
    /// remains unrevised, and a complete penal profile or stage is not required.
    ///
    /// Commit root, revision, receipt and audit together. Withdrawal preserves all
    /// values and source snapshots, admits nothing, and is terminal for that root.
    /// Correct may change any selected source within the notification's fixed
    /// resolution parent. No mutation cascades to other declarations or sources.
    fn commit(
        &self,
        actor: UserId,
        case_id: CaseId,
        prepared: PreparedFactChange,
    ) -> Result<FactDetail, ApplicationError>;
}

/// Authentication and permissions precede preparation and every read. Submission
/// reauthenticates the same actor after admission, compares the exact preparation
/// digest and delegates the final authorization check to the audited transaction.
/// Declared temporal precision is preserved without a no-future policy or inferred
/// ordering. Preparing and submitting do not select legal rules or trigger work.
pub trait ProceduralFactWorkflow: Send + Sync {
    fn list_resolutions(
        &self,
        token: &str,
        case_id: CaseId,
        query: ResolutionQuery,
    ) -> Result<ResolutionPage, ApplicationError>;
    fn list_notifications(
        &self,
        token: &str,
        case_id: CaseId,
        resolution_id: ResolutionId,
        query: NotificationQuery,
    ) -> Result<NotificationPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        target: FactTarget,
        revision: Option<FactRevision>,
    ) -> Result<FactDetail, ApplicationError>;
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        target: FactTarget,
        query: FactHistoryQuery,
    ) -> Result<FactHistoryPage, ApplicationError>;
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: ProceduralFactCommand,
    ) -> Result<FactDraft, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: ProceduralFactCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<FactDetail, ApplicationError>;
}
