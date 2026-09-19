use super::read_support as read;
use application::{
    deadline_reevaluation::PredecessorReceipt,
    deadline_worker::{
        DeadlineWorkerAttempt, DeadlineWorkerBase, DeadlineWorkerErrorCode,
        DeadlineWorkerFailureKind,
    },
    deadlines::DeadlineId,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, typed_participants::Uuid};
use postgres::{Row, Transaction};

const BOUNDS: &str = "octet_length(failure_kind) BETWEEN 1 AND 32
    AND octet_length(error_code) BETWEEN 1 AND 32
    AND (checked_base_submission_digest IS NULL OR octet_length(checked_base_submission_digest)=32)
    AND (checked_base_capture_digest IS NULL OR octet_length(checked_base_capture_digest)=32)";
const COLUMNS: &str = "attempt_id,job_id,attempt_number,checked_base_revision,
    checked_base_submission_digest,checked_base_capture_digest,failure_kind,error_code,
    failed_at_seconds,failed_at_nanoseconds,retry_at_seconds,retry_at_nanoseconds";

/// Read the latest durable failure without certifying that its cause was valid.
pub(crate) fn latest(
    tx: &mut Transaction<'_>,
    id: Uuid,
    hasher: &dyn DocumentHasher,
) -> Result<Option<DeadlineWorkerAttempt>, ApplicationError> {
    let selected = tx.query_opt(
        "SELECT attempt_id FROM deadline_reevaluation_attempts WHERE job_id=$1 ORDER BY attempt_number DESC LIMIT 1",
        &[&id],
    ).map_err(read::port)?;
    let Some(row) = selected else { return Ok(None) };
    let attempt_id = row.try_get("attempt_id").map_err(read::inconsistent)?;
    let attempt = by_id(tx, attempt_id, hasher)?
        .ok_or_else(|| read::inconsistent("latest immutable worker attempt disappeared"))?;
    if attempt.job_id != id {
        return Err(read::inconsistent(
            "latest worker attempt belongs to another job",
        ));
    }
    Ok(Some(attempt))
}

/// Reconcile a stable attempt identity after an uncertain registration reply.
/// Job association and any claimed verified base are authenticated; failure
/// evidence is not required to pass the execution checks that originally failed.
pub(crate) fn by_id(
    tx: &mut Transaction<'_>,
    id: Uuid,
    hasher: &dyn DocumentHasher,
) -> Result<Option<DeadlineWorkerAttempt>, ApplicationError> {
    let Some(row) = bounded_row(tx, id)? else {
        return Ok(None);
    };
    if row
        .try_get::<_, Uuid>("attempt_id")
        .map_err(read::inconsistent)?
        != id
    {
        return Err(read::inconsistent("worker attempt identity differs"));
    }
    let job_id: Uuid = row.try_get("job_id").map_err(read::inconsistent)?;
    let number: i64 = row.try_get("attempt_number").map_err(read::inconsistent)?;
    if number <= 0 {
        return Err(read::inconsistent("worker attempt number must be positive"));
    }
    let job = tx
        .query_opt(
            "SELECT deadline_id,case_id FROM deadline_reevaluation_jobs WHERE id=$1",
            &[&job_id],
        )
        .map_err(read::port)?
        .ok_or_else(|| read::inconsistent("worker attempt job is absent"))?;
    if number > 1 && !tx.query_one(
        "SELECT EXISTS(SELECT 1 FROM deadline_reevaluation_attempts WHERE job_id=$1 AND attempt_number=$2)",
        &[&job_id, &(number - 1)],
    ).map_err(read::port)?.try_get::<_, bool>(0).map_err(read::inconsistent)? {
        return Err(read::inconsistent("worker attempt predecessor is absent"));
    }
    let checked_base = checked_base(tx, &row, &job, hasher)?;
    let (failure_kind, error_code) = failure(&row)?;
    let failed_at = read::instant(&row, "failed_at_seconds", "failed_at_nanoseconds")?;
    let retry_at = read::instant(&row, "retry_at_seconds", "retry_at_nanoseconds")?;
    if retry_at <= failed_at {
        return Err(read::inconsistent(
            "worker retry time must follow its failure",
        ));
    }
    Ok(Some(DeadlineWorkerAttempt {
        attempt_id: id,
        job_id,
        attempt_number: u64::try_from(number).map_err(read::inconsistent)?,
        checked_base,
        failure_kind,
        error_code,
        failed_at,
        retry_at,
    }))
}

fn bounded_row(tx: &mut Transaction<'_>, id: Uuid) -> Result<Option<Row>, ApplicationError> {
    let row = tx.query_opt(&format!(
        "SELECT coalesce(({BOUNDS}),false) AS bounded FROM deadline_reevaluation_attempts WHERE attempt_id=$1"
    ), &[&id]).map_err(read::port)?;
    let Some(row) = row else { return Ok(None) };
    if !row
        .try_get::<_, bool>("bounded")
        .map_err(read::inconsistent)?
    {
        return Err(read::inconsistent("worker attempt fields exceed bounds"));
    }
    tx.query_opt(
        &format!(
        "SELECT {COLUMNS} FROM deadline_reevaluation_attempts WHERE attempt_id=$1 AND ({BOUNDS})"
    ),
        &[&id],
    )
    .map_err(read::port)?
    .map(Some)
    .ok_or_else(|| read::inconsistent("immutable worker attempt changed during read"))
}

fn checked_base(
    tx: &mut Transaction<'_>,
    row: &Row,
    job: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<Option<DeadlineWorkerBase>, ApplicationError> {
    let revision: Option<i64> = row
        .try_get("checked_base_revision")
        .map_err(read::inconsistent)?;
    let submission: Option<&[u8]> = row
        .try_get("checked_base_submission_digest")
        .map_err(read::inconsistent)?;
    let capture: Option<&[u8]> = row
        .try_get("checked_base_capture_digest")
        .map_err(read::inconsistent)?;
    let saved = match (revision, submission, capture) {
        (None, None, None) => return Ok(None),
        (Some(revision), Some(submission), Some(capture)) => DeadlineWorkerBase {
            revision: read::revision(revision)?,
            receipt: PredecessorReceipt {
                submission_digest: read::digest(submission)?,
                capture_digest: read::digest(capture)?,
            },
        },
        _ => return Err(read::inconsistent("worker attempt checked base is partial")),
    };
    let base = crate::deadline_postgres::storage::detail(
        tx,
        CaseId::from_uuid(job.try_get("case_id").map_err(read::inconsistent)?),
        DeadlineId::from_uuid(job.try_get("deadline_id").map_err(read::inconsistent)?),
        Some(saved.revision),
        hasher,
    )
    .map_err(read::stored)?;
    read::matches_base(&saved, &base)?;
    Ok(Some(saved))
}

fn failure(
    row: &Row,
) -> Result<(DeadlineWorkerFailureKind, DeadlineWorkerErrorCode), ApplicationError> {
    use DeadlineWorkerErrorCode as Code;
    use DeadlineWorkerFailureKind as Kind;
    let kind = match row
        .try_get::<_, &str>("failure_kind")
        .map_err(read::inconsistent)?
    {
        "transient" => Kind::Transient,
        "inconsistent" => Kind::Inconsistent,
        _ => return Err(read::inconsistent("unknown worker failure kind")),
    };
    let (code, expected_kind) = match row
        .try_get::<_, &str>("error_code")
        .map_err(read::inconsistent)?
    {
        "lock_unavailable" => (Code::LockUnavailable, Kind::Transient),
        "database_unavailable" => (Code::DatabaseUnavailable, Kind::Transient),
        "transaction_interrupted" => (Code::TransactionInterrupted, Kind::Transient),
        "execution_failed" => (Code::ExecutionFailed, Kind::Transient),
        "invalid_stored_evidence" => (Code::InvalidStoredEvidence, Kind::Inconsistent),
        "invalid_durable_job" => (Code::InvalidDurableJob, Kind::Inconsistent),
        _ => return Err(read::inconsistent("unknown worker failure code")),
    };
    if kind != expected_kind {
        return Err(read::inconsistent("worker failure kind and code disagree"));
    }
    Ok((kind, code))
}
