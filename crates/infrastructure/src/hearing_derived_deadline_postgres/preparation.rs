use super::port;
use application::{
    deadlines::*, documents::StageSupportReadLimits, hearing_derived_deadlines::*,
    hearing_results::HearingResultChange, identity::Principal, ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    actor: Principal,
    case: CaseId,
    command: &HearingDerivedDeadlineCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<HearingDerivedDeadlineInputs, ApplicationError> {
    let (deadline, _) = command.deadline.clone().into_parts();
    let DeadlineChange::Register { definition } = &deadline.change else {
        return Err(DeadlineError::Invalid("derived consequence requires register").into());
    };
    if !matches!(command.result.change, HearingResultChange::Record { .. }) {
        return Err(DeadlineError::Invalid("derived consequence requires record").into());
    }
    let result = crate::hearing_result_postgres::preparation::load(
        tx,
        case,
        &command.result,
        limits,
        hasher,
    )?;
    let used: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_deadline_revisions WHERE operation_id=$1)",
            &[&deadline.operation_id.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if used {
        return Err(DeadlineError::OperationConflict.into());
    }
    let exists: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_deadlines WHERE id=$1)",
            &[&deadline.deadline_id.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if exists {
        return Err(DeadlineError::RevisionConflict.into());
    }
    let scope = tx
        .query_opt(
            "SELECT case_id FROM deadline_profiles WHERE id=$1",
            &[&definition.profile.id.as_uuid()],
        )
        .map_err(port)?
        .ok_or(DeadlineError::ProfileUnavailable)?;
    if scope
        .get::<_, Option<uuid::Uuid>>(0)
        .is_some_and(|id| id != case.as_uuid())
    {
        return Err(DeadlineError::ProfileUnavailable.into());
    }
    let profile = crate::deadline_profile_postgres::storage::detail(
        tx,
        definition.profile.id,
        Some(definition.profile.revision),
        hasher,
    )?;
    let profile_head =
        crate::deadline_profile_postgres::storage::detail(tx, definition.profile.id, None, hasher)?;
    let (calendar, calendar_head) = match definition.input.calendar {
        Some(selected) => (
            Some(crate::judicial_calendar_postgres::storage::detail(
                tx,
                selected.id,
                Some(selected.revision),
                hasher,
            )?),
            Some(crate::judicial_calendar_postgres::storage::detail(
                tx,
                selected.id,
                None,
                hasher,
            )?),
        ),
        None => (None, None),
    };
    let responsible =
        crate::deadline_postgres::authorization::responsible(tx, definition.responsible, case)?;
    Ok(HearingDerivedDeadlineInputs {
        actor,
        result,
        profile,
        profile_head,
        calendar,
        calendar_head,
        responsible,
    })
}
