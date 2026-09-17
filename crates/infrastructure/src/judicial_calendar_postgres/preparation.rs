use super::{port, storage};
use application::{judicial_calendars::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    command: &JudicialCalendarCommand,
    hasher: &dyn DocumentHasher,
) -> Result<JudicialCalendarPreparation, ApplicationError> {
    command.result_revision()?;
    if tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM judicial_calendar_revisions WHERE operation_id=$1)",
            &[&command.operation_id.as_uuid()],
        )
        .map_err(port)?
        .get::<_, bool>(0)
    {
        return Err(JudicialCalendarError::OperationConflict.into());
    }
    let base = match storage::detail(tx, command.calendar_id, None, hasher) {
        Ok(value) => Some(value),
        Err(ApplicationError::JudicialCalendar(JudicialCalendarError::NotFound)) => None,
        Err(error) => return Err(error),
    };
    match (&command.change, &base) {
        (JudicialCalendarChange::Publish { .. }, Some(_)) => {
            return Err(JudicialCalendarError::RevisionConflict.into())
        }
        (JudicialCalendarChange::Replace { .. } | JudicialCalendarChange::Retire { .. }, None) => {
            return Err(JudicialCalendarError::NotFound.into())
        }
        _ => {}
    }
    let initial_scope = if let Some(base) = &base {
        if base.revision.get() != command.expected_revision() {
            return Err(JudicialCalendarError::RevisionConflict.into());
        }
        if base.status != JudicialCalendarStatus::Published {
            return Err(JudicialCalendarError::Retired.into());
        }
        if let JudicialCalendarChange::Replace { values, .. } = &command.change {
            if values.scope() != base.values.scope() {
                return Err(JudicialCalendarError::ScopeChangeForbidden.into());
            }
        }
        Some(base.values.scope().clone())
    } else {
        None
    };
    Ok(JudicialCalendarPreparation {
        calendar_id: command.calendar_id,
        base,
        initial_scope,
    })
}
