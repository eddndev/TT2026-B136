use super::{port, sources, storage};
use application::{procedural_resources::ResourceStatus, resource_activities::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher, identity::UserId};
use postgres::Transaction;

pub(super) fn replay(
    tx: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
    resource: ResourceId,
    command: &ResourceActivityCommand,
    hasher: &dyn DocumentHasher,
) -> Result<Option<ResourceActivityDetail>, ApplicationError> {
    let row = tx.query_opt("SELECT association_id,case_id,resource_id,revision FROM case_resource_activity_association_revisions WHERE operation_id=$1",
        &[&command.operation_id.as_uuid()]).map_err(port)?;
    let Some(row) = row else { return Ok(None) };
    if row.get::<_, uuid::Uuid>("association_id") != command.association_id.as_uuid()
        || row.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
        || row.get::<_, uuid::Uuid>("resource_id") != resource.as_uuid()
    {
        return Err(ResourceActivityError::OperationConflict.into());
    }
    let detail = storage::detail(
        tx,
        case,
        resource,
        command.association_id,
        Some(storage::revision(row.get("revision"))?),
        hasher,
    )?;
    if detail.recorded_by.id != actor || resource_activity_command_from_detail(&detail)? != *command
    {
        return Err(ResourceActivityError::OperationConflict.into());
    }
    Ok(Some(detail))
}
pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    resource: ResourceId,
    command: &ResourceActivityCommand,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceActivityMaterial, ApplicationError> {
    command.result_revision()?;
    crate::postgres_case_status::require_active(tx, case)?;
    let resource_head =
        crate::procedural_resource_postgres::storage::detail(tx, case, resource, None, hasher)?;
    if resource_head.revision != command.expected_resource_revision {
        return Err(ResourceActivityError::ResourceRevisionConflict.into());
    }
    let base = match &command.change {
        ResourceActivityChange::Link { .. } => {
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
            None
        }
        ResourceActivityChange::Unlink {
            expected_revision, ..
        } => {
            let base = storage::detail(tx, case, resource, command.association_id, None, hasher)?;
            if base.revision != *expected_revision {
                return Err(ResourceActivityError::RevisionConflict.into());
            }
            if base.status != ResourceActivityStatus::Linked {
                return Err(ResourceActivityError::StateUnchanged.into());
            }
            Some(base)
        }
    };
    let selected = match (&command.change, &base) {
        (ResourceActivityChange::Link { selection }, None) => *selection,
        (ResourceActivityChange::Unlink { .. }, Some(base)) => base.selection,
        _ => return Err(ResourceActivityError::RevisionConflict.into()),
    };
    if selected.resource.revision > resource_head.revision
        || selected
            .act
            .is_some_and(|act| act.resource_revision > resource_head.revision)
    {
        return Err(ResourceActivityError::SourceMismatch.into());
    }
    let sources = sources::load(tx, case, resource, selected, hasher)?;
    if base.as_ref().is_some_and(|base| base.sources != sources) {
        return Err(ResourceActivityError::SourceMismatch.into());
    }
    let administration = crate::cases::storage::detail(tx, case, hasher)?.administration;
    Ok(ResourceActivityMaterial {
        case_id: case,
        base,
        administration,
        resource_head,
        sources,
    })
}
