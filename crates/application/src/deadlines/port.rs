use super::*;
use crate::ApplicationError;
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};

/// Case authorization precedes every lookup and pagination, including empty pages.
/// Closed cases remain readable. Every projection must match the immutable complete
/// record, canonical inputs, calculation and receipt; reads do not rerun arithmetic.
pub trait DeadlineStore: Send + Sync {
    fn list(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: DeadlineQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlinePage, ApplicationError>;
    fn get(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: DeadlineId,
        revision: Option<DeadlineRevision>,
        at: OffsetDateTime,
    ) -> Result<DeadlineDetail, ApplicationError>;
    fn history(
        &self,
        actor: UserId,
        case_id: CaseId,
        id: DeadlineId,
        query: DeadlineHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineHistoryPage, ApplicationError>;
    /// Resolve the exact selected sources and their separate current heads together.
    fn prepare(
        &self,
        actor: UserId,
        case_id: CaseId,
        command: &DeadlineCommand,
    ) -> Result<DeadlinePreparation, ApplicationError>;
    /// Under one audited transaction, reauthorize the active actor, assigned case and
    /// responsible account, check current heads and base revision, and reproduce the
    /// confirmed reviewed state. Source changes conflict; no maximum or date fallback
    /// may replace an unavailable input. Capture the definitive clock after the lock.
    /// Register/correct may capture a newer active administration without changing the
    /// review digest. Attention/retirement retain the historical calculation exactly.
    /// State, receipt, audit and derived events either commit together or roll back.
    fn commit(
        &self,
        actor: UserId,
        prepared: PreparedDeadlineChange,
    ) -> Result<DeadlineDetail, ApplicationError>;
}

pub trait DeadlineWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: DeadlineQuery,
    ) -> Result<DeadlinePage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: DeadlineId,
        revision: Option<DeadlineRevision>,
    ) -> Result<DeadlineDetail, ApplicationError>;
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: DeadlineId,
        query: DeadlineHistoryQuery,
    ) -> Result<DeadlineHistoryPage, ApplicationError>;
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: DeadlineCommand,
    ) -> Result<DeadlineDraft, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: DeadlineCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<DeadlineDetail, ApplicationError>;
}
