use super::{inconsistent, port, write};
use application::{measure_corrections::MeasureAdministrativeCapture, ApplicationError};
use domain::{
    audit::{chain_digest, GENESIS_PREVIOUS},
    crypto::{DocumentHasher, Sha256Digest},
    precautionary_measures::MeasureCorrectionOperationId,
};
use postgres::Transaction;

pub(super) fn verify(
    tx: &mut Transaction<'_>,
    group: &MeasureAdministrativeCapture,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let review = &group.review;
    let row = tx
        .query_opt(
            "SELECT o.audit_sequence FROM case_measure_operations o
         JOIN case_measure_administrations d ON d.operation_id=o.operation_id AND d.case_id=o.case_id
            AND d.capture_digest=o.owner_digest
         WHERE o.operation_id=$1 AND o.case_id=$2
            AND o.family='a1' AND o.owner_digest=$3",
            &[
                &review.command.operation_id.as_uuid(),
                &review.case_id.as_uuid(),
                &group.capture_digest.as_bytes().as_slice(),
            ],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("administrative group has no exact audit association"))?;
    let sequence: i64 = row.get("audit_sequence");
    if sequence < 0 {
        return Err(inconsistent("administrative audit sequence is negative"));
    }
    let marker = write::marker(group);
    let count: i64 = tx
        .query_one(
            "SELECT count(*) FROM (SELECT sequence FROM audit_events
         WHERE resource=$1 AND action='measure_administrative.recorded' LIMIT 2) matching",
            &[&marker],
        )
        .map_err(port)?
        .get(0);
    if count != 1 {
        return Err(inconsistent(
            "administrative mutation marker is absent or duplicated",
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
        .ok_or_else(|| inconsistent("administrative audit event is absent or exceeds bounds"))?;
    let timestamp: String = row.get("timestamp");
    let entry = crate::audit_postgres::decode_event(row).map_err(inconsistent)?;
    if entry.event.sequence != sequence as u64
        || entry.event.resource != marker
        || entry.event.action != "measure_administrative.recorded"
        || entry.event.actor != review.actor.email
        || entry.event.timestamp != group.recorded_at
        || entry.event.timestamp.offset() != time::UtcOffset::UTC
        || entry.event.timestamp_rfc3339()? != timestamp
    {
        return Err(inconsistent(
            "administrative audit event differs from the exact original group",
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
                inconsistent("administrative audit predecessor is absent or malformed")
            })?;
        let bytes: Vec<u8> = row.get("chain");
        Sha256Digest::from_bytes(&bytes).map_err(inconsistent)?
    };
    if chain_digest(hasher, &previous, &entry.event)? != entry.chain {
        return Err(inconsistent(
            "administrative audit chain commitment differs",
        ));
    }
    Ok(())
}

/// Original mutation evidence prevents reuse of a missing administrative owner.
pub(super) fn operation_absent(
    tx: &mut Transaction<'_>,
    operation: MeasureCorrectionOperationId,
) -> Result<(), ApplicationError> {
    let pattern = format!("ma1:case:%:operation:{operation}:measure:%");
    let exists: bool = tx.query_one(
        "SELECT EXISTS(SELECT 1 FROM audit_events WHERE action='measure_administrative.recorded' AND resource LIKE $1)",
        &[&pattern],
    ).map_err(port)?.get(0);
    if exists {
        return Err(inconsistent(
            "absent administrative owner retains its original audit marker",
        ));
    }
    Ok(())
}
