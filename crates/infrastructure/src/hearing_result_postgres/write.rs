use super::port;
use application::{hearing_results::*, ApplicationError};
use postgres::Transaction;

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    detail: &HearingResultDetail,
    command: &HearingResultCommand,
) -> Result<(), ApplicationError> {
    let s = &detail.snapshot;
    if command.action() == HearingResultAction::Record {
        let c = s.continuation;
        let values = c.map(|v| v.values_digest.as_bytes().to_vec());
        let submission = c.map(|v| v.submission_digest.as_bytes().to_vec());
        tx.execute("INSERT INTO case_hearing_results(id,case_id,hearing_id,anchor_revision,anchor_values_digest,anchor_submission_digest,
            continuation_hearing_id,continuation_result_id,continuation_revision,continuation_values_digest,continuation_submission_digest)
            VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            &[&s.id.as_uuid(),&s.case_id.as_uuid(),&s.hearing_id.as_uuid(),&i64::from(s.anchor.revision.get()),
            &s.anchor.values_digest.as_bytes().as_slice(),&s.anchor.submission_digest.as_bytes().as_slice(),
            &c.map(|v|v.hearing_id.as_uuid()),&c.map(|v|v.result_id.as_uuid()),&c.map(|v|i64::from(v.revision.get())),&values,&submission]).map_err(port)?;
    }
    let canonical = s.values.canonical_bytes();
    let submission = hearing_result_submission_bytes(
        s.recorded_by.id,
        s.case_id,
        command,
        &s.anchor,
        s.continuation.as_ref(),
        s.values_digest,
    );
    tx.execute("INSERT INTO case_hearing_result_revisions(result_id,case_id,hearing_id,revision,values_canonical,values_digest,
        operation_id,action,reason,submission_canonical,submission_digest,recorded_administration_revision,
        recorded_administration_digest,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email,
        support_name,support_format,support_policy) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
        &[&s.id.as_uuid(),&s.case_id.as_uuid(),&s.hearing_id.as_uuid(),&i64::from(s.revision.get()),&canonical,&s.values_digest.as_bytes().as_slice(),
        &s.receipt.operation_id.as_uuid(),&s.receipt.action.as_str(),&s.reason.as_ref().map(HearingResultText::as_str),&submission,&s.receipt.submission_digest.as_bytes().as_slice(),
        &i64::from(s.recorded_administration_revision.get()),&s.recorded_administration_digest.as_bytes().as_slice(),&s.recorded_at.unix_timestamp(),&(s.recorded_at.nanosecond() as i32),
        &s.recorded_by.id.as_uuid(),&s.recorded_by.email,&detail.support.as_ref().map(|s|s.name.as_str()),&detail.support.as_ref().map(|s|s.format.as_str()),&detail.support.as_ref().map(|s|s.policy.as_str())]).map_err(port)?;
    Ok(())
}
