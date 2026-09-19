use application::{
    deadline_reevaluation::PredecessorReceipt,
    deadline_worker::DeadlineWorkerBase,
    deadlines::{DeadlineError, DeadlineRevision},
    ApplicationError,
};
use domain::{clock::OffsetDateTime, crypto::Sha256Digest};
use postgres::{Row, Transaction};

pub(super) fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineError::StoredInconsistent(error.to_string()).into()
}

pub(super) fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("deadline worker history database", error)
}

pub(super) fn stored(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::Port(_) | ApplicationError::ClassifiedPort { .. } => error,
        _ => inconsistent(error),
    }
}

pub(super) fn instant(
    row: &Row,
    seconds: &str,
    nanos: &str,
) -> Result<OffsetDateTime, ApplicationError> {
    let at = OffsetDateTime::from_unix_timestamp(row.try_get(seconds).map_err(inconsistent)?)
        .map_err(inconsistent)?
        .replace_nanosecond(
            u32::try_from(row.try_get::<_, i32>(nanos).map_err(inconsistent)?)
                .map_err(inconsistent)?,
        )
        .map_err(inconsistent)?;
    if !(1..=9999).contains(&at.year()) {
        return Err(inconsistent(
            "worker history timestamp year is outside bounds",
        ));
    }
    Ok(at)
}

pub(super) fn revision(raw: i64) -> Result<DeadlineRevision, ApplicationError> {
    DeadlineRevision::new(u32::try_from(raw).map_err(inconsistent)?).map_err(inconsistent)
}

pub(super) fn digest(bytes: &[u8]) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(bytes).map_err(inconsistent)
}

pub(super) fn base(
    row: &Row,
    number: &str,
    submission: &str,
    capture: &str,
) -> Result<DeadlineWorkerBase, ApplicationError> {
    Ok(DeadlineWorkerBase {
        revision: revision(row.try_get(number).map_err(inconsistent)?)?,
        receipt: PredecessorReceipt {
            submission_digest: digest(row.try_get(submission).map_err(inconsistent)?)?,
            capture_digest: digest(row.try_get(capture).map_err(inconsistent)?)?,
        },
    })
}

pub(super) fn matches_base(
    saved: &DeadlineWorkerBase,
    detail: &application::deadlines::DeadlineDetail,
) -> Result<(), ApplicationError> {
    if saved.revision != detail.revision
        || saved.receipt.submission_digest != detail.receipt.submission_digest
        || saved.receipt.capture_digest != detail.receipt.capture_digest
    {
        return Err(inconsistent("worker history base commitments differ"));
    }
    Ok(())
}

pub(super) fn no_operation_revision(
    tx: &mut Transaction<'_>,
    operation: application::deadlines::DeadlineOperationId,
) -> Result<(), ApplicationError> {
    if tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_deadline_revisions WHERE operation_id=$1)",
            &[&operation.as_uuid()],
        )
        .map_err(port)?
        .try_get::<_, bool>(0)
        .map_err(inconsistent)?
    {
        return Err(inconsistent(
            "no-change job operation has a persisted revision",
        ));
    }
    Ok(())
}
