use super::port;
use application::{hearings::*, ApplicationError};
use postgres::Transaction;

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    detail: &HearingDetail,
    command: &HearingCommand,
) -> Result<(), ApplicationError> {
    let s = &detail.snapshot;
    let c = s.scheduling_context;
    if command.action() == HearingAction::Schedule {
        tx.execute(
            "INSERT INTO case_hearings(id,case_id) VALUES($1,$2)",
            &[&s.id.as_uuid(), &s.case_id.as_uuid()],
        )
        .map_err(port)?;
    }
    let canonical = s.values.canonical_bytes();
    let submission =
        hearing_submission_bytes(s.recorded_by.id, s.case_id, command, s.values_digest);
    let stage_digest = c.stage_digest.map(|digest| digest.as_bytes().to_vec());
    tx.execute("INSERT INTO case_hearing_revisions(hearing_id,case_id,revision,values_canonical,values_digest,
        operation_id,action,reason,submission_canonical,submission_digest,scheduling_administration_revision,
        scheduling_administration_digest,scheduling_stage_revision,scheduling_stage,scheduling_stage_digest,
        recorded_administration_revision,recorded_administration_digest,recorded_at_seconds,recorded_at_nanoseconds,
        recorded_by,recorded_by_email,support_name,support_format,support_policy)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)",
        &[&s.id.as_uuid(),&s.case_id.as_uuid(),&i64::from(s.revision.get()),&canonical,&s.values_digest.as_bytes().as_slice(),
          &s.receipt.operation_id.as_uuid(),&s.receipt.action.as_str(),&s.reason.as_ref().map(HearingNote::as_str),
          &submission,&s.receipt.submission_digest.as_bytes().as_slice(),&i64::from(c.administration_revision.get()),
          &c.administration_digest.as_bytes().as_slice(),&i64::from(c.stage_revision.get()),&c.stage.as_str(),&stage_digest,
          &i64::from(s.recorded_administration_revision.get()),&s.recorded_administration_digest.as_bytes().as_slice(),
          &s.recorded_at.unix_timestamp(),&(s.recorded_at.nanosecond() as i32),&s.recorded_by.id.as_uuid(),&s.recorded_by.email,
          &detail.support.as_ref().map(|support|support.name.as_str()),&detail.support.as_ref().map(|support|support.format.as_str()),
          &detail.support.as_ref().map(|support|support.policy.as_str())]).map_err(port)?;
    Ok(())
}
