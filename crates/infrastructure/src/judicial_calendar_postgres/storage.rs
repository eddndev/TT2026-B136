use super::{decode, inconsistent, port};
use application::{judicial_calendars::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    id: JudicialCalendarId,
    revision: Option<JudicialCalendarRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<JudicialCalendarDetail, ApplicationError> {
    let selected = raw(tx, id, revision, hasher)?;
    let initial = if selected.revision.get() == 1 {
        selected.clone()
    } else {
        raw(
            tx,
            id,
            Some(JudicialCalendarRevision::new(1).map_err(inconsistent)?),
            hasher,
        )
        .map_err(source_error)?
    };
    if initial.receipt.action != JudicialCalendarAction::Publish
        || selected.values.scope() != initial.values.scope()
    {
        return Err(inconsistent(
            "calendar scope differs from its initial revision",
        ));
    }
    if selected.revision.get() > 1 {
        let prior = raw(
            tx,
            id,
            Some(JudicialCalendarRevision::new(selected.revision.get() - 1).map_err(inconsistent)?),
            hasher,
        )
        .map_err(source_error)?;
        if prior.status != JudicialCalendarStatus::Published
            || prior.values.scope() != initial.values.scope()
            || (selected.status == JudicialCalendarStatus::Retired
                && (selected.values != prior.values
                    || selected.values_digest != prior.values_digest))
        {
            return Err(inconsistent("calendar successor or retired values differ"));
        }
    }
    Ok(selected)
}
fn raw(
    tx: &mut Transaction<'_>,
    id: JudicialCalendarId,
    revision: Option<JudicialCalendarRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<JudicialCalendarDetail, ApplicationError> {
    let number = revision.map(|v| i64::from(v.get()));
    let row = tx
        .query_opt(
            "SELECT r.*,c.initial_revision FROM judicial_calendar_revisions r
        JOIN judicial_calendars c ON c.id=r.calendar_id WHERE r.calendar_id=$1
        AND ($2::bigint IS NULL OR r.revision=$2) ORDER BY r.revision DESC LIMIT 1",
            &[&id.as_uuid(), &number],
        )
        .map_err(port)?;
    match row {
        Some(row) => decode::row(&row, hasher),
        None => {
            if revision.is_none()
                && tx
                    .query_opt(
                        "SELECT id FROM judicial_calendars WHERE id=$1",
                        &[&id.as_uuid()],
                    )
                    .map_err(port)?
                    .is_some()
            {
                return Err(inconsistent("calendar root has no initial revision"));
            }
            Err(JudicialCalendarError::NotFound.into())
        }
    }
}
fn source_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::JudicialCalendar(JudicialCalendarError::NotFound) => {
            inconsistent("calendar historical revision is missing")
        }
        other => other,
    }
}
