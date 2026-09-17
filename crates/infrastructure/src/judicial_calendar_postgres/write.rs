use super::port;
use application::{judicial_calendars::*, ApplicationError};
use postgres::Transaction;

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    v: &JudicialCalendarDetail,
    command: &JudicialCalendarCommand,
) -> Result<(), ApplicationError> {
    if command.action() == JudicialCalendarAction::Publish {
        tx.execute(
            "INSERT INTO judicial_calendars(id) VALUES($1)",
            &[&v.id.as_uuid()],
        )
        .map_err(port)?;
    }
    let canonical = v.values.canonical_bytes();
    let submission = judicial_calendar_submission_bytes(v.recorded_by.id, command, v.values_digest);
    tx.execute("INSERT INTO judicial_calendar_revisions(calendar_id,revision,values_canonical,values_digest,
        operation_id,action,reason,submission_canonical,submission_digest,recorded_at_seconds,
        recorded_at_nanoseconds,recorded_by,recorded_by_email) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        &[&v.id.as_uuid(),&i64::from(v.revision.get()),&canonical,&v.values_digest.as_bytes().as_slice(),
            &v.receipt.operation_id.as_uuid(),&v.receipt.action.as_str(),&v.reason.as_ref().map(JudicialCalendarReason::as_str),
            &submission,&v.receipt.submission_digest.as_bytes().as_slice(),&v.recorded_at.unix_timestamp(),
            &(v.recorded_at.nanosecond() as i32),&v.recorded_by.id.as_uuid(),&v.recorded_by.email]).map_err(port)?;
    Ok(())
}
