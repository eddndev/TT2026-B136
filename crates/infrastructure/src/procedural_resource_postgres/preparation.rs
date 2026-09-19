use super::{inconsistent, port, sources, storage};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, procedural_resources::*,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, identity::UserId};
use postgres::Transaction;

pub(super) fn replay(
    tx: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
    command: &ResourceCommand,
    hasher: &dyn DocumentHasher,
) -> Result<Option<ResourceDetail>, ApplicationError> {
    let row=tx.query_opt("SELECT resource_id,case_id,revision FROM case_procedural_resource_revisions WHERE operation_id=$1",&[&command.operation_id.as_uuid()]).map_err(port)?;
    let Some(row) = row else { return Ok(None) };
    if row.get::<_, uuid::Uuid>(0) != command.resource_id.as_uuid()
        || row.get::<_, uuid::Uuid>(1) != case.as_uuid()
    {
        return Err(ProceduralResourceError::OperationConflict.into());
    }
    let detail = storage::detail(
        tx,
        case,
        command.resource_id,
        Some(storage::revision(row.get(2))?),
        hasher,
    )?;
    if detail.recorded_by.id != actor || resource_command_from_detail(&detail)? != *command {
        return Err(ProceduralResourceError::OperationConflict.into());
    }
    Ok(Some(detail))
}
pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &ResourceCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceMaterial, ApplicationError> {
    command.result_revision()?;
    crate::postgres_case_status::require_active(tx, case)?;
    let base = match storage::detail(tx, case, command.resource_id, None, hasher) {
        Ok(detail) => Some(detail),
        Err(ApplicationError::ProceduralResource(ProceduralResourceError::NotFound)) => None,
        Err(error) => return Err(error),
    };
    if base.as_ref().map(|b| b.revision.get()).unwrap_or(0) != command.expected_revision() {
        return Err(ProceduralResourceError::RevisionConflict.into());
    }
    if let Some(base) = &base {
        match (base.status, command.action()) {
            (ResourceStatus::Archived, ResourceAction::Reactivate) => {}
            (ResourceStatus::Archived, _) => return Err(ProceduralResourceError::Archived.into()),
            (ResourceStatus::Active, ResourceAction::Reactivate) => {
                return Err(ProceduralResourceError::StateUnchanged.into())
            }
            _ => {}
        }
    }
    let act_base = match &command.change {
        ResourceChange::RecordAct { act_id, .. } => {
            if tx
                .query_opt(
                    "SELECT id FROM case_procedural_resource_acts WHERE id=$1",
                    &[&act_id.as_uuid()],
                )
                .map_err(port)?
                .is_some()
            {
                return Err(ProceduralResourceError::RevisionConflict.into());
            }
            None
        }
        ResourceChange::CorrectAct {
            act_id,
            expected_act_revision,
            ..
        } => {
            let row=tx.query_opt("SELECT revision,act_revision FROM case_procedural_resource_revisions WHERE case_id=$1 AND resource_id=$2 AND act_id=$3 ORDER BY act_revision DESC LIMIT 1",
                &[&case.as_uuid(),&command.resource_id.as_uuid(),&act_id.as_uuid()]).map_err(port)?.ok_or(ProceduralResourceError::NotFound)?;
            if row.get::<_, i64>(1) != i64::from(expected_act_revision.get()) {
                return Err(ProceduralResourceError::RevisionConflict.into());
            }
            Some(storage::detail(
                tx,
                case,
                command.resource_id,
                Some(storage::revision(row.get(0))?),
                hasher,
            )?)
        }
        _ => None,
    };
    let (resolution, appellants) = match &command.change {
        ResourceChange::Register { values } | ResourceChange::Correct { values, .. } => {
            sources::material(tx, case, values, hasher)?
        }
        _ => (None, vec![]),
    };
    let selected = match &command.change {
        ResourceChange::Register { values } | ResourceChange::Correct { values, .. } => {
            values.direct_supports()
        }
        ResourceChange::RecordAct { values, .. } | ResourceChange::CorrectAct { values, .. } => {
            values.direct_supports()
        }
        _ => vec![],
    };
    if selected.len() > 2 {
        return Err(inconsistent("resource direct support limit exceeded"));
    }
    let retained = match command.action() {
        ResourceAction::Correct => base
            .as_ref()
            .map(|b| b.sources.supports.as_slice())
            .unwrap_or(&[]),
        ResourceAction::CorrectAct => act_base
            .as_ref()
            .and_then(|b| b.act.as_ref())
            .map(|a| a.supports.as_slice())
            .unwrap_or(&[]),
        _ => &[],
    };
    sources::validate_supports(tx, case, retained)?;
    let mut records = vec![];
    for support in selected {
        if let Some(old) = retained.iter().find(|s| s.reference == support.reference()) {
            if old.digest != support.digest() {
                return Err(ApplicationError::StageSupportDigestMismatch);
            }
        } else {
            tx.query_opt(
                "SELECT 1 FROM documents WHERE case_id=$1 AND id=$2 AND version=$3 FOR SHARE",
                &[
                    &case.as_uuid(),
                    &support.reference().id.as_uuid(),
                    &i64::from(support.reference().version.get()),
                ],
            )
            .map_err(port)?
            .ok_or_else(|| {
                ApplicationError::DocumentNotFound(support.reference().id.to_string())
            })?;
            records.push(crate::case_stages::documents::load(
                tx,
                case,
                StageSupportRef::new(support.reference(), support.digest()),
                limits,
            )?);
        }
    }
    records.sort_by_key(|r| (r.id.as_uuid(), r.version.get()));
    Ok(ResourceMaterial {
        case_id: case,
        base,
        act_base,
        administration: crate::cases::storage::detail(tx, case, hasher)?.administration,
        stage: crate::case_stages::query::current(tx, case, hasher)?,
        resolution,
        appellants,
        records,
    })
}
