use crate::{
    deadline_reevaluation::{Observations, PredecessorReceipt},
    deadline_technical::{DeadlineReevaluationCommand, DeadlineReevaluationNoChange},
    deadlines::{DeadlineId, DeadlineReceipt, DeadlineRevision},
};
use domain::{
    case_administration::CaseRevision, cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest,
    typed_participants::Uuid,
};

/// One committed completion, a durable failed attempt, or no currently eligible job.
/// Idle does not assert that every job succeeded or that every deadline is current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineWorkerRun {
    Idle,
    Completed(Box<DeadlineWorkerResult>),
    Deferred(DeadlineWorkerAttempt),
}

/// Immutable completion authenticated by the store against its durable job.
/// The command retains the job's stable operation and exact technical cause,
/// whose job identity must match job_id. Captured times are canonical UTC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineWorkerResult {
    pub job_id: Uuid,
    pub case_id: CaseId,
    pub deadline_id: DeadlineId,
    pub command: DeadlineReevaluationCommand,
    pub base: DeadlineWorkerBase,
    pub outcome: DeadlineWorkerOutcome,
    pub completed_at: OffsetDateTime,
}

/// Both commitments identify the exact historical base examined by the worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineWorkerBase {
    pub revision: DeadlineRevision,
    pub receipt: PredecessorReceipt,
}

/// A completion references its new revision or explains why no revision was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineWorkerOutcome {
    Revision {
        revision: DeadlineRevision,
        receipt: Box<DeadlineReceipt>,
    },
    NoChange {
        reason: DeadlineReevaluationNoChange,
        checked: Option<DeadlineWorkerChecked>,
    },
}

/// Exact observations and full administration evidence examined for a no-change
/// outcome. Retired and already-initialized outcomes omit this value because
/// those decisions need only the authenticated base and cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineWorkerChecked {
    pub observations: Observations,
    pub administration_revision: Option<CaseRevision>,
    pub administration_evidence_digest: Sha256Digest,
}

/// A failed execution remains pending. Attempts are immutable, positively
/// numbered within 1..=i64::MAX for the job. Both times are canonical UTC and
/// retry_at must be later than failed_at.
/// A checked base is present only if its complete receipt was verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineWorkerAttempt {
    pub attempt_id: Uuid,
    pub job_id: Uuid,
    pub attempt_number: u64,
    pub checked_base: Option<DeadlineWorkerBase>,
    pub failure_kind: DeadlineWorkerFailureKind,
    pub error_code: DeadlineWorkerErrorCode,
    pub failed_at: OffsetDateTime,
    pub retry_at: OffsetDateTime,
}

/// Inconsistent data must still reject a new store opening until repaired.
/// Durable scheduling never bypasses the startup inventory validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineWorkerFailureKind {
    Transient,
    Inconsistent,
}

/// Stable diagnostic categories omit raw database messages and sensitive data.
/// InvalidStoredEvidence and InvalidDurableJob are inconsistent failures; all
/// other codes describe transient failures. The store rejects mismatched pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineWorkerErrorCode {
    LockUnavailable,
    DatabaseUnavailable,
    TransactionInterrupted,
    InvalidStoredEvidence,
    InvalidDurableJob,
    ExecutionFailed,
}
