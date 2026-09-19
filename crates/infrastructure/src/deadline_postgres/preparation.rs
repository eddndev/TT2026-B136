use super::{authorization, inconsistent, port, storage};
use application::{
    deadline_profiles::*,
    deadlines::*,
    procedural_facts::{FactDetail, FactTarget},
    ApplicationError,
};
use domain::{
    cases::CaseId, crypto::DocumentHasher, deadline_triggers::TriggerSourceRef,
    procedural_facts::FactDeclaration,
};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &DeadlineCommand,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlinePreparation, ApplicationError> {
    command.result_revision()?;
    let exists = authorization::visible(tx, case, command.deadline_id)?;
    let used: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_deadline_revisions WHERE operation_id=$1)",
            &[&command.operation_id.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if used {
        return Err(DeadlineError::OperationConflict.into());
    }
    let base = if exists {
        Some(storage::detail(
            tx,
            case,
            command.deadline_id,
            None,
            hasher,
        )?)
    } else {
        None
    };
    if matches!(command.change, DeadlineChange::Register { .. }) {
        if base.is_some() {
            return Err(DeadlineError::RevisionConflict.into());
        }
    } else {
        let current = base.as_ref().ok_or(DeadlineError::NotFound)?;
        if current.revision.get() != command.expected_revision() {
            return Err(DeadlineError::RevisionConflict.into());
        }
        if current.status == DeadlineStatus::Retired {
            return Err(DeadlineError::Retired.into());
        }
    }
    let administration = crate::cases::storage::detail(tx, case, hasher)?.administration;
    let (resolved, responsible) = match &command.change {
        DeadlineChange::Register { definition } | DeadlineChange::Correct { definition, .. } => {
            if definition.input.selection.case_id != case {
                return Err(DeadlineError::Invalid("input.case_id").into());
            }
            let responsible = authorization::responsible(tx, definition.responsible, case)?;
            let scoped = tx
                .query_opt(
                    "SELECT case_id FROM deadline_profiles WHERE id=$1",
                    &[&definition.profile.id.as_uuid()],
                )
                .map_err(port)?
                .ok_or(DeadlineError::ProfileUnavailable)?;
            if scoped
                .try_get::<_, Option<uuid::Uuid>>(0)
                .map_err(inconsistent)?
                .is_some_and(|id| id != case.as_uuid())
            {
                return Err(DeadlineError::ProfileUnavailable.into());
            }
            let profile = crate::deadline_profile_postgres::storage::detail(
                tx,
                definition.profile.id,
                Some(definition.profile.revision),
                hasher,
            )
            .map_err(profile_error)?;
            let profile_head = crate::deadline_profile_postgres::storage::detail(
                tx,
                definition.profile.id,
                None,
                hasher,
            )
            .map_err(profile_error)?;
            let material = crate::deadline_input_postgres::load_material(
                tx,
                &definition.input.selection,
                definition.input.calendar,
                hasher,
            )?;
            (
                Some(DeadlineResolvedInputs {
                    profile,
                    profile_head,
                    material,
                }),
                Some(responsible),
            )
        }
        _ => (None, None),
    };
    Ok(DeadlinePreparation {
        case_id: case,
        deadline_id: command.deadline_id,
        administration,
        base,
        resolved,
        responsible,
    })
}
fn profile_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::DeadlineProfile(DeadlineProfileError::NotFound) => {
            DeadlineError::ProfileUnavailable.into()
        }
        other => other,
    }
}

/// Resolve the current parent independently of a notification's selected historical parent.
/// The caller authorizes the case and shares the audited lock with all source writers.
pub(super) fn notification_parent_head(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &DeadlineCommand,
    hasher: &dyn DocumentHasher,
) -> Result<Option<FactDetail>, ApplicationError> {
    let definition = match &command.change {
        DeadlineChange::Register { definition } | DeadlineChange::Correct { definition, .. } => {
            definition
        }
        _ => return Ok(None),
    };
    if definition.input.selection.case_id != case {
        return Err(DeadlineError::Invalid("input.case_id").into());
    }
    let FactDeclaration::Known(TriggerSourceRef::Notification { resolution, .. }) =
        &definition.input.selection.source
    else {
        return Ok(None);
    };
    crate::procedural_fact_postgres::storage::detail(
        tx,
        case,
        FactTarget::Resolution(resolution.id),
        None,
        hasher,
    )
    .map(Some)
}
