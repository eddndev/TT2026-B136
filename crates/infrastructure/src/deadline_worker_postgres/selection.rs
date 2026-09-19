use super::{inconsistent, port};
use application::{deadlines::DeadlineOperationId, ApplicationError};
use domain::typed_participants::Uuid;
use postgres::Transaction;
use time::OffsetDateTime;

#[derive(Clone, Copy)]
pub(super) struct Target {
    pub id: Uuid,
    pub operation_id: DeadlineOperationId,
}

/// Select one eligible job under the shared audited mutation lock. The SQL
/// limits materialized rows, not the number of pending jobs it may examine.
pub(super) fn next(
    tx: &mut Transaction<'_>,
    at: OffsetDateTime,
) -> Result<Option<Target>, ApplicationError> {
    let row = tx.query_opt(
        "SELECT j.id,j.operation_id FROM deadline_reevaluation_jobs j
         LEFT JOIN LATERAL (
             SELECT a.attempt_id,a.checked_base_revision,a.checked_base_submission_digest,
                 a.checked_base_capture_digest,a.retry_at_seconds,a.retry_at_nanoseconds
             FROM deadline_reevaluation_attempts a WHERE a.job_id=j.id
             ORDER BY a.attempt_number DESC LIMIT 1
         ) a ON true
         WHERE NOT EXISTS(SELECT 1 FROM deadline_reevaluation_results r WHERE r.job_id=j.id)
         AND (a.attempt_id IS NULL OR
             ROW(a.retry_at_seconds,a.retry_at_nanoseconds)<=ROW($1::bigint,$2::integer)
             OR (a.checked_base_revision IS NOT NULL AND EXISTS (
                 SELECT 1 FROM case_deadline_revisions d WHERE d.deadline_id=j.deadline_id
                 AND d.revision=(SELECT max(h.revision) FROM case_deadline_revisions h
                     WHERE h.deadline_id=j.deadline_id)
                 AND ROW(d.revision,d.submission_digest,d.capture_digest) IS DISTINCT FROM
                     ROW(a.checked_base_revision,a.checked_base_submission_digest,a.checked_base_capture_digest))))
         ORDER BY j.created_at_seconds,j.created_at_nanoseconds,j.id LIMIT 1",
        &[&at.unix_timestamp(), &(at.nanosecond() as i32)],
    ).map_err(port)?;
    row.map(|row| {
        Ok(Target {
            id: row.try_get("id").map_err(inconsistent)?,
            operation_id: DeadlineOperationId::from_uuid(
                row.try_get("operation_id").map_err(inconsistent)?,
            ),
        })
    })
    .transpose()
}
