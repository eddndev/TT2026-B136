use super::{inconsistent, port, write};
use application::{precautionary_hearings::PrecautionaryHearingCapture, ApplicationError};
use domain::{
    audit::{chain_digest, GENESIS_PREVIOUS},
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    precautionary_hearings::{PrecautionaryHearingId, PrecautionaryHearingOperationId},
};
use postgres::Transaction;

pub(crate) fn verify(
    tx: &mut Transaction<'_>,
    capture: &PrecautionaryHearingCapture,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let review = &capture.review;
    let row = tx
        .query_opt(
            "SELECT audit_sequence FROM case_precautionary_hearing_revisions
             WHERE hearing_id=$1 AND case_id=$2 AND revision=$3 AND operation_id=$4",
            &[
                &review.command.hearing_id.as_uuid(),
                &review.case_id.as_uuid(),
                &i64::from(review.result_revision.get()),
                &review.command.operation_id.as_uuid(),
            ],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("precautionary capture has no audit association"))?;
    let sequence: i64 = row.get("audit_sequence");
    if sequence < 0 {
        return Err(inconsistent("precautionary audit sequence is negative"));
    }
    let marker = write::marker(capture);
    let occurrences: i64 = tx
        .query_one(
            "SELECT count(*) FROM (SELECT sequence FROM audit_events
             WHERE resource=$1 AND action IN ('precautionary_hearing.schedule',
             'precautionary_hearing.replace','precautionary_hearing.cancel') LIMIT 2) matching",
            &[&marker],
        )
        .map_err(port)?
        .get(0);
    if occurrences != 1 {
        return Err(inconsistent(
            "precautionary audit marker is absent or duplicated",
        ));
    }
    let row = tx
        .query_opt(
            "SELECT sequence,timestamp,actor,action,resource,chain FROM audit_events
             WHERE sequence=$1 AND octet_length(timestamp) BETWEEN 1 AND 64
             AND octet_length(actor) BETWEEN 1 AND 1280
             AND octet_length(action) BETWEEN 1 AND 64
             AND octet_length(resource) BETWEEN 1 AND 512 AND octet_length(chain)=32",
            &[&sequence],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("precautionary audit event is absent or exceeds bounds"))?;
    let timestamp: String = row.get("timestamp");
    let entry = crate::audit_postgres::decode_event(row).map_err(inconsistent)?;
    let action = format!("precautionary_hearing.{}", review.command.action().as_str());
    if entry.event.sequence != sequence as u64
        || entry.event.resource != marker
        || entry.event.action != action
        || entry.event.actor != review.actor.email
        || entry.event.timestamp != capture.recorded_at
        || entry.event.timestamp_rfc3339()? != timestamp
    {
        return Err(inconsistent(
            "precautionary audit event differs from exact capture",
        ));
    }
    let previous = if sequence == 0 {
        GENESIS_PREVIOUS
    } else {
        let row = tx
            .query_opt(
                "SELECT chain FROM audit_events WHERE sequence=$1 AND octet_length(chain)=32",
                &[&(sequence - 1)],
            )
            .map_err(port)?
            .ok_or_else(|| {
                inconsistent("precautionary audit predecessor is absent or malformed")
            })?;
        let bytes: Vec<u8> = row.get("chain");
        Sha256Digest::from_bytes(&bytes).map_err(inconsistent)?
    };
    if chain_digest(hasher, &previous, &entry.event)? != entry.chain {
        return Err(inconsistent("precautionary audit chain commitment differs"));
    }
    Ok(())
}

/// A hearing operation cannot be reused after losing the row that its audit committed.
pub(super) fn operation_absent(
    tx: &mut Transaction<'_>,
    case: CaseId,
    operation: PrecautionaryHearingOperationId,
) -> Result<(), ApplicationError> {
    let pattern = format!("ph1:%:operation:{operation}:%");
    let exists: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM audit_events WHERE resource LIKE $1)",
            &[&pattern],
        )
        .map_err(port)?
        .get(0);
    if exists {
        return Err(inconsistent(format!(
            "precautionary operation {operation} requested in case {case} has an orphan audit marker"
        )));
    }
    Ok(())
}

/// Retained audit evidence prevents reuse of a missing appointment root.
pub(super) fn hearing_absent(
    tx: &mut Transaction<'_>,
    id: PrecautionaryHearingId,
) -> Result<(), ApplicationError> {
    let pattern = format!("ph1:%:hearing:{id}:operation:%");
    let exists: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM audit_events WHERE resource LIKE $1)",
            &[&pattern],
        )
        .map_err(port)?
        .get(0);
    if exists {
        return Err(inconsistent("absent hearing retains an audit origin"));
    }
    Ok(())
}

/// A surviving earlier prefix cannot replace a lost terminal revision.
pub(super) fn hearing_intact(
    tx: &mut Transaction<'_>,
    id: PrecautionaryHearingId,
) -> Result<(), ApplicationError> {
    let pattern = format!("ph1:%:hearing:{id}:operation:%");
    let orphan: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM audit_events a
         LEFT JOIN case_precautionary_hearing_revisions r
         ON r.audit_sequence=a.sequence AND r.hearing_id=$2
         WHERE a.resource LIKE $1 AND a.action IN ('precautionary_hearing.schedule',
         'precautionary_hearing.replace','precautionary_hearing.cancel')
         AND r.audit_sequence IS NULL)",
            &[&pattern, &id.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if orphan {
        return Err(inconsistent(
            "hearing history is missing a committed audit revision",
        ));
    }
    Ok(())
}

/// Pagination must not hide an appointment whose root was lost after opening.
pub(crate) fn case_intact(tx: &mut Transaction<'_>, case: CaseId) -> Result<(), ApplicationError> {
    let pattern = format!("ph1:case:{case}:hearing:%");
    let orphan: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM audit_events a
         LEFT JOIN case_precautionary_hearing_revisions r
         ON r.audit_sequence=a.sequence AND r.case_id=$2
         LEFT JOIN case_precautionary_hearings h
         ON h.id=r.hearing_id AND h.case_id=r.case_id
         WHERE a.resource LIKE $1 AND a.action IN ('precautionary_hearing.schedule',
         'precautionary_hearing.replace','precautionary_hearing.cancel')
         AND h.id IS NULL)",
            &[&pattern, &case.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if orphan {
        return Err(inconsistent(
            "case has an orphan precautionary hearing audit",
        ));
    }
    Ok(())
}
