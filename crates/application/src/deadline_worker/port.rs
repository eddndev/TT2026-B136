use super::{DeadlineWorkerAttempt, DeadlineWorkerResult, DeadlineWorkerRun};
use crate::ApplicationError;
use domain::typed_participants::Uuid;

/// Consume durable jobs with the existing technical identity and audit lock.
/// No caller-supplied user, operation, cause, result or attempt authorizes work.
/// The adapter authenticates the persisted job and base before preparation.
///
/// Revision, result and completion audit commit together. A no-change result
/// retains its exact cause, historical base and any observations it examined.
/// A failed execution rolls back; a separately audited durable attempt may
/// defer it while other eligible jobs proceed. Failure to record that attempt
/// returns an error instead of reporting a completion or successful deferral.
///
/// Reopening always validates the complete stored inventory. A transient fault
/// with valid data can recover after reopen; persistent corruption still blocks
/// startup until repaired. An applicable retry delay is tied to the exact base
/// commitments, so a newer human revision cannot be overwritten by a stale job.
/// Stable job and operation identities allow reconciliation after lost replies.
/// The adapter releases the transaction lock after each job or failed attempt.
pub trait DeadlineWorkerStore: Send + Sync {
    /// Return only a committed result, a durable attempt, or an unchanged poll.
    /// Idle includes jobs whose applicable retry times have not yet arrived.
    fn run_next(&self) -> Result<DeadlineWorkerRun, ApplicationError>;

    /// Read a completion verified against its exact job, cause, base, examined
    /// evidence and produced revision. None means no completion, including an
    /// absent job. Invalid stored evidence returns an error, never None.
    /// Reading appends no audit and does not reevaluate or mutate progress.
    fn result(&self, job_id: Uuid) -> Result<Option<DeadlineWorkerResult>, ApplicationError>;

    /// Read the greatest durable attempt number, or None if none exists.
    /// Verify the job association, ledger fields and any claimed checked base.
    /// The attempt does not certify that its job passed execution validation.
    /// Reading appends no audit and does not modify the retry schedule.
    fn latest_attempt(
        &self,
        job_id: Uuid,
    ) -> Result<Option<DeadlineWorkerAttempt>, ApplicationError>;
}
