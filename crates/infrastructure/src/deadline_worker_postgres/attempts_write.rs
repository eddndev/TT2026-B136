use super::{inconsistent, port};
use application::{deadline_worker::*, deadlines::DeadlineOperationId, ApplicationError};
use postgres::Transaction;
use serde_json::json;

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    value: &DeadlineWorkerAttempt,
) -> Result<(), ApplicationError> {
    let revision = value.checked_base.map(|b| i64::from(b.revision.get()));
    let submission = value
        .checked_base
        .as_ref()
        .map(|b| b.receipt.submission_digest.as_bytes().as_slice());
    let capture = value
        .checked_base
        .as_ref()
        .map(|b| b.receipt.capture_digest.as_bytes().as_slice());
    let (kind, code) = tags(value);
    tx.execute(
        "INSERT INTO deadline_reevaluation_attempts(
        attempt_id,job_id,attempt_number,checked_base_revision,checked_base_submission_digest,
        checked_base_capture_digest,failure_kind,error_code,failed_at_seconds,failed_at_nanoseconds,
        retry_at_seconds,retry_at_nanoseconds) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        &[
            &value.attempt_id,
            &value.job_id,
            &i64::try_from(value.attempt_number).map_err(inconsistent)?,
            &revision,
            &submission,
            &capture,
            &kind,
            &code,
            &value.failed_at.unix_timestamp(),
            &(value.failed_at.nanosecond() as i32),
            &value.retry_at.unix_timestamp(),
            &(value.retry_at.nanosecond() as i32),
        ],
    )
    .map_err(port)?;
    Ok(())
}

pub(super) fn audit(
    tx: &mut Transaction<'_>,
    operation: DeadlineOperationId,
    value: &DeadlineWorkerAttempt,
) -> Result<(), ApplicationError> {
    let (kind, code) = tags(value);
    let checked = value.checked_base.as_ref().map(|b| {
        json!({
            "revision": b.revision.get(), "submission": b.receipt.submission_digest.to_hex(),
            "capture": b.receipt.capture_digest.to_hex(),
        })
    });
    let resource = json!({
        "job": value.job_id.to_string(), "operation": operation.to_string(),
        "attempt": value.attempt_id.to_string(), "number": value.attempt_number,
        "checked_base": checked, "kind": kind, "code": code,
        "failed_at_seconds": value.failed_at.unix_timestamp(),
        "failed_at_nanoseconds": value.failed_at.nanosecond(),
        "retry_at_seconds": value.retry_at.unix_timestamp(),
        "retry_at_nanoseconds": value.retry_at.nanosecond(),
    });
    crate::audit_postgres::append_transaction(
        tx,
        "deadline_reevaluator",
        "deadline.reevaluation_deferred",
        &resource.to_string(),
        value.failed_at,
    )?;
    Ok(())
}

fn tags(value: &DeadlineWorkerAttempt) -> (&'static str, &'static str) {
    use DeadlineWorkerErrorCode as Code;
    let kind = match value.failure_kind {
        DeadlineWorkerFailureKind::Transient => "transient",
        DeadlineWorkerFailureKind::Inconsistent => "inconsistent",
    };
    let code = match value.error_code {
        Code::LockUnavailable => "lock_unavailable",
        Code::DatabaseUnavailable => "database_unavailable",
        Code::TransactionInterrupted => "transaction_interrupted",
        Code::InvalidStoredEvidence => "invalid_stored_evidence",
        Code::InvalidDurableJob => "invalid_durable_job",
        Code::ExecutionFailed => "execution_failed",
    };
    (kind, code)
}
