use super::port;
use application::{
    deadlines::*, procedural_resources::ResourceStatus, resource_activities::*,
    resource_deadlines::*, ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;
pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    resource: ResourceId,
    command: &ResourceDeadlineCommand,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceDeadlineMaterial, ApplicationError> {
    let (deadline, _) = command.deadline.clone().into_parts();
    if !matches!(deadline.change, DeadlineChange::Register { .. }) {
        return Err(DeadlineError::Invalid("contextual creation requires register").into());
    }
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
    {
        return Err(ResourceActivityError::RevisionConflict.into());
    }
    if command.resource.revision > resource_head.revision
        || command
            .act
            .is_some_and(|act| act.resource_revision > resource_head.revision)
    {
        return Err(ResourceActivityError::SourceMismatch.into());
    }
    let (source, act) = crate::resource_activity_postgres::sources::load_resource(
        tx,
        case,
        resource,
        command.resource,
        command.act,
        hasher,
    )?;
    let deadline = crate::deadline_postgres::preparation::load(tx, case, &deadline, hasher)?;
    Ok(ResourceDeadlineMaterial {
        case_id: case,
        administration: deadline.administration.clone(),
        resource_head,
        resource: source,
        act,
        deadline,
    })
}
