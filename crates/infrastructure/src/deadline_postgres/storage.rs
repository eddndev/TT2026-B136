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
    AND octet_length(r.review_canonical) BETWEEN 5 AND 524288
    AND octet_length(r.capture_canonical) BETWEEN 5 AND 524288
    AND octet_length(r.submission_canonical) BETWEEN 107 AND 4115
    AND octet_length(r.review_digest)=32 AND octet_length(r.capture_digest)=32
    AND octet_length(r.submission_digest)=32 AND octet_length(r.title)<=800
    AND COALESCE(octet_length(r.reason),0)<=4000
    AND octet_length(r.responsible_email)<=1280 AND octet_length(r.recorded_by_email)<=1280
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
        if prior.status != DeadlineStatus::Active {
            return Err(inconsistent("deadline history continued after retirement"));
        }
        match selected.receipt.action {
            DeadlineAction::Correct if selected.attention != prior.attention => {
                return Err(inconsistent("deadline correction changed attention"));
            }
            DeadlineAction::SetAttention | DeadlineAction::Retire => {
                if selected.receipt.action == DeadlineAction::Retire
                    && selected.attention != prior.attention
                {
                    return Err(inconsistent("deadline retirement changed attention"));
                }
                let mut copied = selected.clone();
                copied.status = prior.status;
                copied.attention = prior.attention.clone();
                if deadline_review_bytes(hasher, &copied)? != deadline_review_bytes(hasher, &prior)?
                    || deadline_capture_bytes(hasher, &copied)?
                        != deadline_capture_bytes(hasher, &prior)?
                {
                    return Err(inconsistent(
                        "attention or retirement changed captured deadline content",
                    ));
                }
            }
            _ => {}
        }
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
