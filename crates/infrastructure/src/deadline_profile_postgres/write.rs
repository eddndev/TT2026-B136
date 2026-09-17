use super::port;
use application::{deadline_profiles::*, ApplicationError};
use postgres::Transaction;
pub(super) fn insert(
    tx: &mut Transaction<'_>,
    v: &DeadlineProfileDetail,
    command: &DeadlineProfileCommand,
) -> Result<(), ApplicationError> {
    if command.action() == DeadlineProfileAction::Publish {
        let case = match v.definition.scope() {
            DeadlineProfileScope::Global(_) => None,
            DeadlineProfileScope::Case(id) => Some(id.as_uuid()),
        };
        tx.execute(
            "INSERT INTO deadline_profiles(id,case_id) VALUES($1,$2)",
            &[&v.id.as_uuid(), &case],
        )
        .map_err(port)?;
    }
    let canonical = deadline_profile_definition_bytes(&v.definition);
    let submission = deadline_profile_submission_bytes(
        v.recorded_by.id,
        command,
        v.algorithm,
        v.definition_digest,
    );
    tx.execute("INSERT INTO deadline_profile_revisions(profile_id,revision,definition_canonical,definition_digest,algorithm,
        operation_id,action,reason,submission_canonical,submission_digest,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        &[&v.id.as_uuid(),&i64::from(v.revision.get()),&canonical,&v.definition_digest.as_bytes().as_slice(),&1i16,
        &v.receipt.operation_id.as_uuid(),&v.receipt.action.as_str(),&v.reason.as_ref().map(|r|r.as_str()),&submission,
        &v.receipt.submission_digest.as_bytes().as_slice(),&v.recorded_at.unix_timestamp(),&(v.recorded_at.nanosecond() as i32),&v.recorded_by.id.as_uuid(),&v.recorded_by.email]).map_err(port)?;
    Ok(())
}
