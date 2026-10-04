use super::{inconsistent, port, storage};
use application::{
    identity::Principal, resource_activities::*, resource_hearings::*, ApplicationError,
};
use domain::{
    audit::{chain_digest, GENESIS_PREVIOUS},
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    resource_hearings::ResourceHearingRevision,
};
use postgres::Transaction;
pub(super) fn marker(result: &ResourceHearingCreation) -> String {
    let o = &result.origin;
    format!(
        "rhl1:case:{}:resource:{}:hearing:{}:association:{}:operation:{}:submission:{}:capture:{}",
        o.case_id,
        o.resource_id,
        o.hearing_id,
        o.association_id,
        o.operation_id,
        o.submission_digest.to_hex(),
        o.capture_digest.to_hex()
    )
}
pub(super) fn load(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    case: CaseId,
    resource: ResourceId,
    command: &ResourceHearingCommand,
    hasher: &dyn DocumentHasher,
) -> Result<Option<ResourceHearingCreation>, ApplicationError> {
    let operation = command.operation_id.as_uuid();
    let h=tx.query_opt("SELECT hearing_id,case_id,resource_id FROM case_resource_hearing_revisions WHERE operation_id=$1",&[&operation]).map_err(port)?;
    let a=tx.query_opt("SELECT association_id,case_id,resource_id,revision FROM case_resource_activity_association_revisions WHERE operation_id=$1",&[&operation]).map_err(port)?;
    let (h, a) = match (h, a) {
        (None, None) => {
            let marker_exists:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM audit_events WHERE action='resource_hearing.registered' AND resource LIKE $1)",
            &[&format!("%:operation:{operation}:%")]).map_err(port)?.get(0);
            if marker_exists {
                return Err(inconsistent(
                    "resource hearing origin has lost its creation",
                ));
            }
            return Ok(None);
        }
        (Some(h), Some(a)) => (h, a),
        _ => return Err(ResourceActivityError::OperationConflict.into()),
    };
    if h.get::<_, uuid::Uuid>("hearing_id") != command.hearing_id.as_uuid()
        || h.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
        || h.get::<_, uuid::Uuid>("resource_id") != resource.as_uuid()
        || a.get::<_, uuid::Uuid>("association_id") != command.association_id.as_uuid()
        || a.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
        || a.get::<_, uuid::Uuid>("resource_id") != resource.as_uuid()
        || a.get::<_, i64>("revision") != 1
    {
        return Err(ResourceActivityError::OperationConflict.into());
    }
    let hearing = storage::detail(
        tx,
        case,
        command.hearing_id,
        Some(ResourceHearingRevision::initial()),
        hasher,
    )?;
    if hearing.review.recorded_by.id != actor.id || hearing.review.command != *command {
        return Err(ResourceActivityError::OperationConflict.into());
    }
    creation(tx, hearing, hasher).map(Some)
}
pub(super) fn creation(
    tx: &mut Transaction<'_>,
    hearing: ResourceHearingDetail,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceHearingCreation, ApplicationError> {
    let d = &hearing.review;
    let origin = ResourceHearingOrigin {
        case_id: d.case_id,
        resource_id: d.command.resource.id,
        hearing_id: d.command.hearing_id,
        operation_id: d.command.operation_id,
        association_id: d.command.association_id,
        submission_digest: d.submission_digest,
        capture_digest: hearing.capture_digest,
    };
    let association = crate::resource_activity_postgres::storage::detail(
        tx,
        d.case_id,
        d.command.resource.id,
        d.command.association_id,
        Some(ResourceActivityRevision::initial()),
        hasher,
    )?;
    let result = ResourceHearingCreation {
        hearing,
        origin,
        association,
    };
    resource_hearing_creation_matches(hasher, &result)?;
    verify_marker(tx, &result, hasher)?;
    Ok(result)
}
fn verify_marker(
    tx: &mut Transaction<'_>,
    result: &ResourceHearingCreation,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let rows=tx.query("SELECT sequence,timestamp,actor,action,resource,chain FROM audit_events WHERE action='resource_hearing.registered' AND resource=$1 ORDER BY sequence LIMIT 2",&[&marker(result)]).map_err(port)?;
    if rows.len() != 1 {
        return Err(ResourceActivityError::OperationConflict.into());
    }
    let entry = crate::audit_postgres::decode_event(rows.into_iter().next().unwrap())?;
    if entry.event.actor != result.hearing.review.recorded_by.email
        || entry.event.timestamp != result.hearing.recorded_at
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
