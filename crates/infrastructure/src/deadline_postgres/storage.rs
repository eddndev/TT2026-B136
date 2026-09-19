use super::{decode, inconsistent, port};
use application::{deadlines::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;

// Probe variable-size fields before materializing their buffers. The same bounds
// are repeated in the data query so privileged concurrent corruption cannot turn
// a successful probe into an unbounded read.
const BOUNDS: &str = "octet_length(r.input_canonical) BETWEEN 48 AND 98897
    AND octet_length(r.result_canonical) BETWEEN 5 AND 3000000
    AND octet_length(r.observed_administration_canonical) BETWEEN 17 AND 16384
    AND octet_length(r.observed_administration_digest)=32
    AND (
        (substring(r.submission_canonical FROM 1 FOR 5)=convert_to('DLTX1','UTF8')
            AND octet_length(r.review_canonical) BETWEEN 5 AND 524288
            AND octet_length(r.capture_canonical) BETWEEN 5 AND 524288
            AND octet_length(r.submission_canonical) BETWEEN 107 AND 4115
            AND r.tracking_canonical IS NULL AND r.observations_canonical IS NULL
            AND r.recorded_by IS NOT NULL AND r.recorded_by_email IS NOT NULL)
        OR (substring(r.submission_canonical FROM 1 FOR 5)=convert_to('DLTX2','UTF8')
            AND octet_length(r.review_canonical) BETWEEN 5 AND 524341
            AND octet_length(r.capture_canonical) BETWEEN 5 AND 524410
            AND octet_length(r.submission_canonical) BETWEEN 151 AND 5502
            AND r.tracking_canonical IS NOT NULL AND r.observations_canonical IS NOT NULL
            AND octet_length(r.tracking_canonical) BETWEEN 102 AND 122
            AND octet_length(r.observations_canonical) BETWEEN 111 AND 446))
    AND octet_length(r.review_digest)=32 AND octet_length(r.capture_digest)=32
    AND octet_length(r.submission_digest)=32 AND octet_length(r.title)<=800
    AND COALESCE(octet_length(r.reason),0)<=4000
    AND octet_length(r.responsible_email)<=1280
    AND ((r.recorded_by IS NULL AND r.recorded_by_email IS NULL)
        OR (r.recorded_by IS NOT NULL AND r.recorded_by_email IS NOT NULL
            AND octet_length(r.recorded_by_email)<=1280))
    AND octet_length(r.attention::text)<=65536
    AND octet_length(r.input_view::text)<=1048576 AND octet_length(r.submission_view::text)<=32768";

pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: DeadlineId,
    revision: Option<DeadlineRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineDetail, ApplicationError> {
    let selected = raw(tx, case, id, revision, hasher)?;
    if selected.revision.get() > 1 {
        let prior = raw(
            tx,
            case,
            id,
            Some(DeadlineRevision::new(selected.revision.get() - 1).map_err(inconsistent)?),
            hasher,
        )
        .map_err(|error| match error {
            ApplicationError::Deadline(DeadlineError::NotFound) => {
                inconsistent("deadline predecessor is missing")
            }
            other => other,
        })?;
        deadline_successor_matches(hasher, &prior, &selected).map_err(inconsistent)?;
    }
    Ok(selected)
}
fn raw(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: DeadlineId,
    revision: Option<DeadlineRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineDetail, ApplicationError> {
    let number = revision.map(|value| i64::from(value.get()));
    let probe = tx.query_opt(&format!("SELECT r.revision,({BOUNDS}) AS bounded
        FROM case_deadline_revisions r JOIN case_deadlines d ON d.id=r.deadline_id AND d.case_id=r.case_id
        WHERE r.case_id=$1 AND r.deadline_id=$2 AND ($3::bigint IS NULL OR r.revision=$3)
        ORDER BY r.revision DESC LIMIT 1"), &[&case.as_uuid(), &id.as_uuid(), &number]).map_err(port)?;
    let Some(probe) = probe else {
        if revision.is_none()
            && tx
                .query_opt(
                    "SELECT id FROM case_deadlines WHERE id=$1 AND case_id=$2",
                    &[&id.as_uuid(), &case.as_uuid()],
                )
                .map_err(port)?
                .is_some()
        {
            return Err(inconsistent("deadline root has no initial revision"));
        }
        return Err(DeadlineError::NotFound.into());
    };
    if !probe.try_get::<_, bool>("bounded").map_err(inconsistent)? {
        return Err(inconsistent("stored deadline fields exceed bounds"));
    }
    let number: i64 = probe.try_get("revision").map_err(inconsistent)?;
    let row = tx.query_opt(&format!("SELECT r.*,d.initial_revision
        FROM case_deadline_revisions r JOIN case_deadlines d ON d.id=r.deadline_id AND d.case_id=r.case_id
        WHERE r.case_id=$1 AND r.deadline_id=$2 AND r.revision=$3 AND ({BOUNDS})"),
        &[&case.as_uuid(), &id.as_uuid(), &number]).map_err(port)?
        .ok_or_else(|| inconsistent("deadline immutable revision changed during read"))?;
    let detail = decode::row(tx, &row, hasher)?;
    if detail.case_id != case || detail.id != id || i64::from(detail.revision.get()) != number {
        return Err(inconsistent(
            "stored deadline identity differs from requested revision",
        ));
    }
    Ok(detail)
}
