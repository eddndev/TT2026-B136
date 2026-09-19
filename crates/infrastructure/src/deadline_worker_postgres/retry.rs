use super::{
    attempts_read, attempts_write, execute::FailureContext, inconsistent, port, results,
    selection::Target,
};
use application::{
    deadline_profiles::DeadlineProfileError, deadline_worker::*, deadlines::DeadlineError,
    hearing_results::HearingResultError, hearings::HearingError,
    judicial_calendars::JudicialCalendarError, procedural_facts::ProceduralFactError,
    ApplicationError, PortFailureKind,
};

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
use domain::{crypto::DocumentHasher, typed_participants::Uuid};
use postgres::Client;
use time::{Duration, OffsetDateTime};

/// Called only after the failed completion transaction has been rolled back.
pub(super) fn record(
    client: &mut Client,
    hasher: &dyn DocumentHasher,
    target: Target,
    context: &FailureContext,
    error: &ApplicationError,
    attempt_id: Uuid,
    at: OffsetDateTime,
) -> Result<DeadlineWorkerRun, ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    if let Some(result) = results::load(&mut tx, target.id, hasher)? {
        tx.commit().map_err(port)?;
        return Ok(DeadlineWorkerRun::Completed(Box::new(result)));
    }
    let previous = attempts_read::latest(&mut tx, target.id, hasher)?;
    let number = previous
        .as_ref()
        .map_or(0, |a| a.attempt_number)
        .checked_add(1)
        .filter(|n| *n <= i64::MAX as u64)
        .ok_or_else(|| inconsistent("worker attempt number exhausted"))?;
    let (failure_kind, error_code) = classify(error, context.verifying_job);
    let base_revision = context.checked_base.map(|b| i64::from(b.revision.get()));
    let prior: i64 = tx
        .query_one(
            "SELECT count(*) FROM deadline_reevaluation_attempts WHERE job_id=$1
         AND checked_base_revision IS NOT DISTINCT FROM $2::bigint",
            &[&target.id, &base_revision],
        )
        .map_err(port)?
        .try_get(0)
        .map_err(inconsistent)?;
    let delay = if failure_kind == DeadlineWorkerFailureKind::Inconsistent {
        3600
    } else {
        let exponent = u32::try_from(prior.min(9)).map_err(inconsistent)?;
        (1_i64 << exponent).min(300)
    };
    let retry_at = at
        .checked_add(Duration::seconds(delay))
        .filter(|next| (1..=9999).contains(&next.year()))
        .ok_or_else(|| inconsistent("worker retry time is outside supported years"))?;
    let attempt = DeadlineWorkerAttempt {
        attempt_id,
        job_id: target.id,
        attempt_number: number,
        checked_base: context.checked_base,
        failure_kind,
        error_code,
        failed_at: at,
        retry_at,
    };
    attempts_write::insert(&mut tx, &attempt)?;
    let actual = attempts_read::by_id(&mut tx, attempt_id, hasher)?
        .ok_or_else(|| inconsistent("inserted worker attempt disappeared"))?;
    if actual != attempt {
        return Err(inconsistent("stored worker attempt differs"));
    }
    attempts_write::audit(&mut tx, target.operation_id, &actual)?;
    tx.commit().map_err(port)?;
    Ok(DeadlineWorkerRun::Deferred(actual))
}

pub(super) fn reconcile(
    client: &mut Client,
    hasher: &dyn DocumentHasher,
    job: Uuid,
    attempt_id: Uuid,
) -> Result<Option<DeadlineWorkerRun>, ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let value = if let Some(result) = results::load(&mut tx, job, hasher)? {
        Some(DeadlineWorkerRun::Completed(Box::new(result)))
    } else if let Some(attempt) = attempts_read::by_id(&mut tx, attempt_id, hasher)? {
        if attempt.job_id != job {
            return Err(inconsistent(
                "reconciled worker attempt belongs to another job",
            ));
        }
        Some(DeadlineWorkerRun::Deferred(attempt))
    } else {
        None
    };
    tx.commit().map_err(port)?;
    Ok(value)
}

fn classify(
    error: &ApplicationError,
    verifying_job: bool,
) -> (DeadlineWorkerFailureKind, DeadlineWorkerErrorCode) {
    use DeadlineWorkerErrorCode as Code;
    use DeadlineWorkerFailureKind as Kind;
    if let ApplicationError::ClassifiedPort { kind, .. } = error {
        return (
            Kind::Transient,
            match kind {
                PortFailureKind::Unavailable => Code::DatabaseUnavailable,
                PortFailureKind::Busy => Code::LockUnavailable,
                PortFailureKind::Interrupted => Code::TransactionInterrupted,
            },
        );
    }
    if matches!(
        error,
        ApplicationError::Deadline(DeadlineError::StoredInconsistent(_))
            | ApplicationError::DeadlineInput(_)
            | ApplicationError::DeadlineProfile(DeadlineProfileError::StoredInconsistent(_))
            | ApplicationError::JudicialCalendar(JudicialCalendarError::StoredInconsistent(_))
            | ApplicationError::ProceduralFact(ProceduralFactError::StoredInconsistent(_))
            | ApplicationError::HearingResult(HearingResultError::StoredInconsistent(_))
            | ApplicationError::Hearing(HearingError::StoredInconsistent(_))
            | ApplicationError::StoredCaseAdministrationInconsistent(_)
            | ApplicationError::StoredCaseStageInconsistent(_)
            | ApplicationError::StoredParticipantInconsistent(_)
    ) {
        (
            Kind::Inconsistent,
            if verifying_job {
                Code::InvalidDurableJob
            } else {
                Code::InvalidStoredEvidence
            },
        )
    } else {
        // Unclassified ports and domain failures do not establish a native category.
        (Kind::Transient, Code::ExecutionFailed)
    }
}
