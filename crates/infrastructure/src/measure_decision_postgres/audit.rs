use super::{inconsistent, port, write};
use application::{precautionary_measures::MeasureDecisionGroupCapture, ApplicationError};
use domain::{
    audit::{chain_digest, GENESIS_PREVIOUS},
    crypto::{DocumentHasher, Sha256Digest},
    precautionary_measures::{MeasureDecisionId, MeasureDecisionOperationId},
};
use postgres::Transaction;

pub(super) fn verify(
    tx: &mut Transaction<'_>,
    group: &MeasureDecisionGroupCapture,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let review = &group.review;
    let row = tx
        .query_opt(
            "SELECT o.audit_sequence FROM case_measure_operations o
         JOIN case_measure_decisions d ON d.operation_id=o.operation_id AND d.case_id=o.case_id
            AND d.group_digest=o.owner_digest
         WHERE o.operation_id=$1 AND o.case_id=$2 AND d.decision_id=$3
            AND o.family='g1' AND o.owner_digest=$4",
            &[
                &review.command.operation_id.as_uuid(),
                &review.case_id.as_uuid(),
                &review.command.decision_id.as_uuid(),
                &group.capture_digest.as_bytes().as_slice(),
            ],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("measure group has no exact audit association"))?;
    let sequence: i64 = row.get("audit_sequence");
    if sequence < 0 {
        return Err(inconsistent("measure audit sequence is negative"));
    }
    let marker = write::marker(group);
    let count: i64 = tx
        .query_one(
            "SELECT count(*) FROM (SELECT sequence FROM audit_events
         WHERE resource=$1 AND action='measure_decision.recorded' LIMIT 2) matching",
            &[&marker],
        )
        .map_err(port)?
        .get(0);
    if count != 1 {
        return Err(inconsistent(
            "measure mutation marker is absent or duplicated",
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
        .ok_or_else(|| inconsistent("measure audit event is absent or exceeds bounds"))?;
    let timestamp: String = row.get("timestamp");
    let entry = crate::audit_postgres::decode_event(row).map_err(inconsistent)?;
    if entry.event.sequence != sequence as u64
        || entry.event.resource != marker
        || entry.event.action != "measure_decision.recorded"
        || entry.event.actor != review.actor.email
        || entry.event.timestamp != group.recorded_at
        || entry.event.timestamp.offset() != time::UtcOffset::UTC
        || entry.event.timestamp_rfc3339()? != timestamp
    {
        return Err(inconsistent(
            "measure audit event differs from the exact original group",
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
            .ok_or_else(|| inconsistent("measure audit predecessor is absent or malformed"))?;
        let bytes: Vec<u8> = row.get("chain");
        Sha256Digest::from_bytes(&bytes).map_err(inconsistent)?
    };
    if chain_digest(hasher, &previous, &entry.event)? != entry.chain {
        return Err(inconsistent("measure audit chain commitment differs"));
    }
    Ok(())
}

/// Original mutation evidence prevents reuse of a lost operation row.
pub(super) fn operation_absent(
    tx: &mut Transaction<'_>,
    operation: MeasureDecisionOperationId,
) -> Result<(), ApplicationError> {
    absent(tx, &format!("mg1:case:%:operation:{operation}:decision:%"))
}

/// Original mutation evidence prevents reuse of a lost immutable decision.
pub(super) fn decision_absent(
    tx: &mut Transaction<'_>,
    decision: MeasureDecisionId,
) -> Result<(), ApplicationError> {
    absent(
        tx,
        &format!("mg1:case:%:operation:%:decision:{decision}:submission:%"),
    )
}

fn absent(tx: &mut Transaction<'_>, pattern: &str) -> Result<(), ApplicationError> {
    let exists: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM audit_events
         WHERE action='measure_decision.recorded' AND resource LIKE $1)",
            &[&pattern],
        )
        .map_err(port)?
        .get(0);
    if exists {
        return Err(inconsistent(
            "absent measure identity retains an original audit marker",
        ));
    }
    Ok(())
}

/// Check retained ownership markers and every bounded advertised member set.
pub(super) fn inventory_intact(tx: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let broken: bool = tx.query_one(
        "SELECT
         EXISTS(SELECT 1 FROM case_measure_operations o
            LEFT JOIN case_measure_decisions d ON d.operation_id=o.operation_id
                AND d.case_id=o.case_id AND d.group_digest=o.owner_digest
            LEFT JOIN audit_events a ON a.sequence=o.audit_sequence
            WHERE o.family<>'g1' OR octet_length(o.owner_digest)<>32 OR o.audit_sequence<0
                OR d.decision_id IS NULL OR a.sequence IS NULL
                OR a.action<>'measure_decision.recorded')
         OR EXISTS(SELECT 1 FROM case_measure_decisions d
            LEFT JOIN case_measure_operations o ON o.operation_id=d.operation_id
                AND o.case_id=d.case_id AND o.owner_digest=d.group_digest
            WHERE o.operation_id IS NULL)
         OR EXISTS(SELECT 1 FROM case_measures m
            LEFT JOIN case_measure_operations o ON o.operation_id=m.root_operation AND o.case_id=m.case_id
            LEFT JOIN case_measure_revisions r ON r.measure_id=m.id AND r.case_id=m.case_id
                AND r.revision=m.initial_revision AND r.owner_operation=m.root_operation
            WHERE m.initial_revision<>1 OR o.operation_id IS NULL OR r.measure_id IS NULL)
         OR EXISTS(SELECT 1 FROM case_measure_revisions r
            LEFT JOIN case_measures m ON m.id=r.measure_id AND m.case_id=r.case_id
            LEFT JOIN case_measure_operations o ON o.operation_id=r.owner_operation AND o.case_id=r.case_id
            WHERE r.revision NOT BETWEEN 1 AND 4294967295 OR r.family<>'m1' OR r.action NOT IN ('impose','confirm','modify','revoke','cease','substitute_out','substitute_in')
                OR m.id IS NULL OR o.operation_id IS NULL)
         OR EXISTS(SELECT 1 FROM audit_events a
            WHERE a.action='measure_decision.recorded' AND NOT EXISTS(
                SELECT 1 FROM case_measure_operations o JOIN case_measure_decisions d
                    ON d.operation_id=o.operation_id AND d.case_id=o.case_id AND d.group_digest=o.owner_digest
                WHERE o.audit_sequence=a.sequence AND CASE WHEN
                    octet_length(d.submission_digest)=32 AND octet_length(d.review_digest)=32
                    AND octet_length(d.decision_digest)=32 AND octet_length(d.group_digest)=32
                    THEN a.resource=
                    'mg1:case:'||d.case_id::text||':operation:'||d.operation_id::text
                    ||':decision:'||d.decision_id::text||':submission:'||encode(d.submission_digest,'hex')
                    ||':review:'||encode(d.review_digest,'hex')||':decision_digest:'||encode(d.decision_digest,'hex')
                    ||':group:'||encode(d.group_digest,'hex') ELSE FALSE END))
         OR EXISTS(SELECT 1 FROM case_measure_decisions d WHERE
            octet_length(d.outcome_view::text)>1048576
            OR octet_length(d.outcome_canonical) NOT BETWEEN 11 AND 645450
            OR d.outcome_digest<>sha256(d.outcome_canonical))",
        &[],
    ).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent(
            "measure ownership inventory is incomplete or contradictory",
        ));
    }
    super::inventory_shape::validate(tx)
}
