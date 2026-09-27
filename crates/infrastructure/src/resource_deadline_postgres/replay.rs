use super::{inconsistent, port};
use application::{
    deadlines::*, identity::Principal, resource_activities::*, resource_deadlines::*,
    ApplicationError,
};
use domain::{
    audit::{chain_digest, GENESIS_PREVIOUS},
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
};
use postgres::Transaction;
pub(super) fn marker(result: &ResourceDeadlineResult) -> String {
    format!(
        "rdl1:case:{}:resource:{}:deadline:{}:association:{}:operation:{}:submission:{}",
        result.deadline.case_id,
        result.association.resource_id,
        result.deadline.id,
        result.association.id,
        result.deadline.receipt.operation_id,
        result.submission_digest.to_hex()
    )
}
pub(super) fn load(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    case: CaseId,
    resource: ResourceId,
    command: &ResourceDeadlineCommand,
    hasher: &dyn DocumentHasher,
) -> Result<Option<ResourceDeadlineResult>, ApplicationError> {
    let deadline = command.deadline.clone().into_parts().0;
    let operation = deadline.operation_id.as_uuid();
    let deadline_row=tx.query_opt("SELECT deadline_id,case_id,revision FROM case_deadline_revisions WHERE operation_id=$1",&[&operation]).map_err(port)?;
    let association_row=tx.query_opt("SELECT association_id,case_id,resource_id,revision FROM case_resource_activity_association_revisions WHERE operation_id=$1",&[&operation]).map_err(port)?;
    let (d, a) = match (deadline_row, association_row) {
        (None, None) => return Ok(None),
        (Some(d), Some(a)) => (d, a),
        _ => return Err(ResourceActivityError::OperationConflict.into()),
    };
    if d.get::<_, uuid::Uuid>("deadline_id") != deadline.deadline_id.as_uuid()
        || d.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
        || d.get::<_, i64>("revision") != 1
        || a.get::<_, uuid::Uuid>("association_id") != command.association_id.as_uuid()
        || a.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
        || a.get::<_, uuid::Uuid>("resource_id") != resource.as_uuid()
        || a.get::<_, i64>("revision") != 1
    {
        return Err(ResourceActivityError::OperationConflict.into());
    }
    let deadline = crate::deadline_postgres::storage::detail(
        tx,
        case,
        deadline.deadline_id,
        Some(DeadlineRevision::initial()),
        hasher,
    )?;
    let association = crate::resource_activity_postgres::storage::detail(
        tx,
        case,
        resource,
        command.association_id,
        Some(ResourceActivityRevision::initial()),
        hasher,
    )?;
    let submission_digest = resource_deadline_submission_digest(
        hasher,
        deadline.receipt.submission_digest,
        association.receipt.submission_digest,
    );
    let result = ResourceDeadlineResult {
        deadline,
        association,
        submission_digest,
    };
    resource_deadline_result_draft(hasher, actor, case, resource, command, &result)?;
    verify_marker(tx, &result, hasher)?;
    Ok(Some(result))
}
fn verify_marker(
    tx: &mut Transaction<'_>,
    result: &ResourceDeadlineResult,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let rows=tx.query("SELECT sequence,timestamp,actor,action,resource,chain FROM audit_events WHERE action='resource_deadline.registered' AND resource=$1 ORDER BY sequence LIMIT 2",&[&marker(result)]).map_err(port)?;
    if rows.len() != 1 {
        return Err(ResourceActivityError::OperationConflict.into());
    }
    let entry = crate::audit_postgres::decode_event(rows.into_iter().next().unwrap())?;
    if entry.event.actor != result.association.recorded_by.email
        || entry.event.timestamp != result.association.recorded_at
    {
        return Err(inconsistent(
            "contextual origin marker author or time differs",
        ));
    }
    let sequence = i64::try_from(entry.event.sequence)
        .map_err(|_| inconsistent("contextual origin sequence overflows"))?;
    let previous = if sequence == 0 {
        GENESIS_PREVIOUS
    } else {
        let row = tx
            .query_opt(
                "SELECT chain FROM audit_events WHERE sequence=$1",
                &[&(sequence - 1)],
            )
            .map_err(port)?
            .ok_or_else(|| inconsistent("contextual origin predecessor is missing"))?;
        let bytes: Vec<u8> = row.get(0);
        Sha256Digest::from_array(
            bytes
                .try_into()
                .map_err(|_| inconsistent("contextual origin predecessor digest is invalid"))?,
        )
    };
    if chain_digest(hasher, &previous, &entry.event)? != entry.chain {
        return Err(inconsistent("contextual origin audit commitment differs"));
    }
    Ok(())
}
