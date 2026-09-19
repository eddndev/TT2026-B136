use super::{read_support as read, results_no_change};
use application::{
    deadline_technical::DeadlineReevaluationCommand,
    deadline_worker::{DeadlineWorkerOutcome, DeadlineWorkerResult},
    deadlines::{DeadlineAction, DeadlineDetail, DeadlineReceiptVersion},
    ApplicationError,
};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher, typed_participants::Uuid};
use postgres::{Row, Transaction};

// Probe before materializing variable-size columns and repeat the bounds in
// the data query, so immutable-row corruption cannot bypass the first check.
const BOUNDS: &str = "octet_length(base_submission_digest)=32
    AND octet_length(base_capture_digest)=32 AND octet_length(outcome) BETWEEN 1 AND 32
    AND (result_submission_digest IS NULL OR octet_length(result_submission_digest)=32)
    AND (result_capture_digest IS NULL OR octet_length(result_capture_digest)=32)
    AND (checked_observations_canonical IS NULL OR octet_length(checked_observations_canonical) BETWEEN 111 AND 446)
    AND (checked_administration_evidence_digest IS NULL OR octet_length(checked_administration_evidence_digest)=32)";
const COLUMNS: &str = "job_id,base_revision,base_submission_digest,base_capture_digest,outcome,
    result_revision,result_submission_digest,result_capture_digest,checked_observations_canonical,
    checked_administration_revision,checked_administration_evidence_digest,
    completed_at_seconds,completed_at_nanoseconds";

/// Read a verified immutable completion without current-head queries or writes.
/// Absence means no stored completion; malformed or orphaned rows are errors.
pub(crate) fn load(
    tx: &mut Transaction<'_>,
    id: Uuid,
    hasher: &dyn DocumentHasher,
) -> Result<Option<DeadlineWorkerResult>, ApplicationError> {
    let Some(row) = bounded_row(tx, id)? else {
        return Ok(None);
    };
    let job = crate::deadline_worker_provenance::load_job(tx, id, hasher)?;
    if row
        .try_get::<_, Uuid>("job_id")
        .map_err(read::inconsistent)?
        != job.id
    {
        return Err(read::inconsistent("worker result job identity differs"));
    }
    let saved = read::base(
        &row,
        "base_revision",
        "base_submission_digest",
        "base_capture_digest",
    )?;
    let base = crate::deadline_postgres::storage::detail(
        tx,
        job.case_id,
        job.deadline_id,
        Some(saved.revision),
        hasher,
    )
    .map_err(read::stored)?;
    read::matches_base(&saved, &base)?;
    let command = DeadlineReevaluationCommand {
        operation_id: job.operation_id,
        cause: job.cause,
    };
    let completed_at = read::instant(&row, "completed_at_seconds", "completed_at_nanoseconds")?;
    let outcome = match row
        .try_get::<_, &str>("outcome")
        .map_err(read::inconsistent)?
    {
        "revision" => produced(tx, &row, &base, &command, completed_at, hasher)?,
        tag => results_no_change::load(tx, &row, &base, &command, tag, hasher)?,
    };
    Ok(Some(DeadlineWorkerResult {
        job_id: job.id,
        case_id: job.case_id,
        deadline_id: job.deadline_id,
        command,
        base: saved,
        outcome,
        completed_at,
    }))
}

fn bounded_row(tx: &mut Transaction<'_>, id: Uuid) -> Result<Option<Row>, ApplicationError> {
    let row = tx.query_opt(&format!(
        "SELECT coalesce(({BOUNDS}),false) AS bounded FROM deadline_reevaluation_results WHERE job_id=$1"
    ), &[&id]).map_err(read::port)?;
    let Some(row) = row else { return Ok(None) };
    if !row
        .try_get::<_, bool>("bounded")
        .map_err(read::inconsistent)?
    {
        return Err(read::inconsistent("worker result fields exceed bounds"));
    }
    tx.query_opt(
        &format!(
            "SELECT {COLUMNS} FROM deadline_reevaluation_results WHERE job_id=$1 AND ({BOUNDS})"
        ),
        &[&id],
    )
    .map_err(read::port)?
    .map(Some)
    .ok_or_else(|| read::inconsistent("immutable worker result changed during read"))
}

fn produced(
    tx: &mut Transaction<'_>,
    row: &Row,
    base: &DeadlineDetail,
    command: &DeadlineReevaluationCommand,
    at: OffsetDateTime,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineWorkerOutcome, ApplicationError> {
    if row
        .try_get::<_, Option<&[u8]>>("checked_observations_canonical")
        .map_err(read::inconsistent)?
        .is_some()
        || row
            .try_get::<_, Option<i64>>("checked_administration_revision")
            .map_err(read::inconsistent)?
            .is_some()
        || row
            .try_get::<_, Option<&[u8]>>("checked_administration_evidence_digest")
            .map_err(read::inconsistent)?
            .is_some()
    {
        return Err(read::inconsistent("revision result has no-change evidence"));
    }
    let revision = read::revision(row.try_get("result_revision").map_err(read::inconsistent)?)?;
    if base.revision.get().checked_add(1) != Some(revision.get()) {
        return Err(read::inconsistent(
            "worker produced revision is not the next revision",
        ));
    }
    let next = crate::deadline_postgres::storage::detail(
        tx,
        base.case_id,
        base.id,
        Some(revision),
        hasher,
    )
    .map_err(read::stored)?;
    if next.receipt.submission_digest
        != read::digest(
            row.try_get("result_submission_digest")
                .map_err(read::inconsistent)?,
        )?
        || next.receipt.capture_digest
            != read::digest(
                row.try_get("result_capture_digest")
                    .map_err(read::inconsistent)?,
            )?
        || next.receipt.operation_id != command.operation_id
        || next.receipt.action != DeadlineAction::Reevaluate
        || next.receipt.expected_revision != base.revision.get()
        || next.recorded_at != at
        || next.recorded_at.offset() != at.offset()
    {
        return Err(read::inconsistent(
            "worker produced receipt or timestamp differs",
        ));
    }
    let DeadlineReceiptVersion::Tracked(metadata) = &next.receipt.version else {
        return Err(read::inconsistent("worker produced a legacy revision"));
    };
    if metadata.cause != Some(command.cause) {
        return Err(read::inconsistent(
            "worker produced cause differs from its job",
        ));
    }
    // storage::detail verifies full receipts, adjacent transition and shared job
    // provenance. Preserve its exact arithmetic capture without recomputing it.
    Ok(DeadlineWorkerOutcome::Revision {
        revision,
        receipt: Box::new(next.receipt),
    })
}
