use super::port;
use application::{
    procedural_resources::ResourceStatus, resource_activities::*, resource_hearings::*,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    resource: ResourceId,
    command: &ResourceHearingCommand,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceHearingMaterial, ApplicationError> {
    crate::postgres_case_status::require_active(tx, case)?;
    let resource_head =
        crate::procedural_resource_postgres::storage::detail(tx, case, resource, None, hasher)?;
    if resource_head.revision != command.expected_resource_revision {
        return Err(ResourceActivityError::ResourceRevisionConflict.into());
    }
    if resource_head.status != ResourceStatus::Active {
        return Err(ResourceActivityError::ResourceArchived.into());
    }
    if tx
        .query_opt(
            "SELECT id FROM case_resource_activity_associations WHERE id=$1",
            &[&command.association_id.as_uuid()],
        )
        .map_err(port)?
        .is_some()
        || tx
            .query_opt(
                "SELECT id FROM case_resource_hearings WHERE id=$1",
                &[&command.hearing_id.as_uuid()],
            )
            .map_err(port)?
            .is_some()
    {
        return Err(ResourceActivityError::RevisionConflict.into());
    }
    let (source, act) = crate::resource_activity_postgres::sources::load_resource(
        tx,
        case,
        resource,
        command.resource,
        command.act,
        hasher,
    )?;
    let mut participants = Vec::with_capacity(command.values.participants().len());
    for selected in command.values.participants() {
        let current =
            crate::participant_postgres::storage::current(tx, case, selected.id(), hasher)?;
        if current.revision_number() != selected.revision() {
            return Err(ResourceActivityError::SourceMismatch.into());
        }
        participants.push(current);
    }
    Ok(ResourceHearingMaterial {
        case_id: case,
        administration: crate::cases::storage::detail(tx, case, hasher)?.administration,
        resource_head,
        resource: source,
        act,
        participants,
    })
}
