use super::{inconsistent, load_job, port, timestamp};
use application::{
    deadline_reevaluation::{TechnicalCause, TechnicalService},
    deadlines::{DeadlineAction, DeadlineActorSnapshot, DeadlineDetail, DeadlineReceiptVersion},
    ApplicationError,
};
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    typed_participants::Uuid,
};
use postgres::Transaction;

/// Authenticate a technical record after its complete canonical receipt has
/// been verified. The enclosing deadline reader also checks its full adjacent
/// predecessor; this function reads only bounded headers and never recursively
/// invokes that reader. Historical linkage does not depend on the current head.
pub(crate) fn validate_technical_record(
    tx: &mut Transaction<'_>,
    detail: &DeadlineDetail,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    if detail.receipt.action != DeadlineAction::Reevaluate {
        return Ok(());
    }
    let DeadlineReceiptVersion::Tracked(metadata) = &detail.receipt.version else {
        return Err(inconsistent("technical record requires a tracked receipt"));
    };
    if detail.recorded_by
        != (DeadlineActorSnapshot::Technical {
            service: TechnicalService::DeadlineReevaluator,
            policy_version: 1,
        })
        || detail.tracking.is_none()
    {
        return Err(inconsistent("technical record author or tracking differs"));
    }
    let cause = metadata
        .cause
        .ok_or_else(|| inconsistent("technical record cause is absent"))?;
    let job_id = match cause {
        TechnicalCause::SourceEvent { job_id, .. }
        | TechnicalCause::LegacyBootstrap { job_id, .. } => job_id,
    };
    let job = load_job(tx, job_id, hasher)?;
    if job.case_id != detail.case_id
        || job.deadline_id != detail.id
        || job.operation_id != detail.receipt.operation_id
        || job.cause != cause
    {
        return Err(inconsistent(
            "technical record differs from authenticated job",
        ));
    }
    let predecessor = metadata
        .predecessor
        .ok_or_else(|| inconsistent("technical predecessor is absent"))?;
    let base_revision = detail.receipt.expected_revision;
    if base_revision == 0 || base_revision.checked_add(1) != Some(detail.revision.get()) {
        return Err(inconsistent("technical predecessor revision differs"));
    }
    let row = tx.query_opt(
        "SELECT v.base_revision,v.base_submission_digest,v.base_capture_digest,
        v.result_revision,v.result_submission_digest,v.result_capture_digest,
        v.completed_at_seconds,v.completed_at_nanoseconds,
        b.submission_digest AS stored_base_submission,b.capture_digest AS stored_base_capture,
        r.submission_digest AS stored_result_submission,r.capture_digest AS stored_result_capture,
        r.operation_id AS stored_operation,r.recorded_at_seconds,r.recorded_at_nanoseconds
        FROM deadline_reevaluation_results v
        JOIN case_deadline_revisions b ON b.case_id=$2 AND b.deadline_id=$3 AND b.revision=v.base_revision
        JOIN case_deadline_revisions r ON r.case_id=$2 AND r.deadline_id=$3 AND r.revision=v.result_revision
        WHERE v.job_id=$1 AND v.outcome='revision'
            AND v.checked_observations_canonical IS NULL
            AND v.checked_administration_revision IS NULL
            AND v.checked_administration_evidence_digest IS NULL
            AND octet_length(v.base_submission_digest)=32 AND octet_length(v.base_capture_digest)=32
            AND octet_length(v.result_submission_digest)=32 AND octet_length(v.result_capture_digest)=32
            AND octet_length(b.submission_digest)=32 AND octet_length(b.capture_digest)=32
            AND octet_length(r.submission_digest)=32 AND octet_length(r.capture_digest)=32",
        &[&job.id, &job.case_id.as_uuid(), &job.deadline_id.as_uuid()],
    ).map_err(port)?.ok_or_else(|| inconsistent("technical record has no matching bounded revision result"))?;
    let digest = |column| -> Result<Sha256Digest, ApplicationError> {
        Sha256Digest::from_bytes(row.try_get::<_, &[u8]>(column).map_err(inconsistent)?)
            .map_err(inconsistent)
    };
    if row
        .try_get::<_, i64>("base_revision")
        .map_err(inconsistent)?
        != i64::from(base_revision)
        || row
            .try_get::<_, i64>("result_revision")
            .map_err(inconsistent)?
            != i64::from(detail.revision.get())
        || digest("base_submission_digest")? != predecessor.submission_digest
        || digest("base_capture_digest")? != predecessor.capture_digest
        || digest("stored_base_submission")? != predecessor.submission_digest
        || digest("stored_base_capture")? != predecessor.capture_digest
        || digest("result_submission_digest")? != detail.receipt.submission_digest
        || digest("result_capture_digest")? != detail.receipt.capture_digest
        || digest("stored_result_submission")? != detail.receipt.submission_digest
        || digest("stored_result_capture")? != detail.receipt.capture_digest
        || row
            .try_get::<_, Uuid>("stored_operation")
            .map_err(inconsistent)?
            != job.operation_id.as_uuid()
    {
        return Err(inconsistent(
            "technical result base or produced commitments differ",
        ));
    }
    let completed = timestamp(
        row.try_get("completed_at_seconds").map_err(inconsistent)?,
        row.try_get("completed_at_nanoseconds")
            .map_err(inconsistent)?,
    )?;
    let recorded = timestamp(
        row.try_get("recorded_at_seconds").map_err(inconsistent)?,
        row.try_get("recorded_at_nanoseconds")
            .map_err(inconsistent)?,
    )?;
    if completed != recorded
        || recorded != detail.recorded_at
        || recorded.offset() != detail.recorded_at.offset()
    {
        return Err(inconsistent(
            "technical completion and recording timestamps differ",
        ));
    }
    Ok(())
}
