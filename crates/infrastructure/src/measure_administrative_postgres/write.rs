use super::{inconsistent, port};
use application::{
    measure_corrections::*, precautionary_measures::MeasureCaptureAction, ApplicationError,
};
use domain::{crypto::DocumentHasher, precautionary_measures::MeasureSupervision};
use postgres::Transaction;

pub(super) fn marker(capture: &MeasureAdministrativeCapture) -> String {
    let review = &capture.review;
    format!(
        "ma1:case:{}:operation:{}:measure:{}:revision:{}:submission:{}:review:{}:capture:{}",
        review.case_id,
        review.command.operation_id,
        review.result.id,
        review.result.revision.get(),
        review.submission_digest.to_hex(),
        review.review_digest.to_hex(),
        capture.capture_digest.to_hex()
    )
}
pub(super) fn action(value: &MeasureAdministrativeAction) -> &'static str {
    match value {
        MeasureAdministrativeAction::Correct(_) => "correct",
        MeasureAdministrativeAction::MarkEnteredInError => "entered_in_error",
    }
}
pub(super) fn validity(value: MeasureCaptureValidity) -> &'static str {
    match value {
        MeasureCaptureValidity::Valid => "valid",
        MeasureCaptureValidity::EnteredInError => "entered_in_error",
    }
}
pub(super) fn retained_action(value: MeasureCaptureAction) -> &'static str {
    match value {
        MeasureCaptureAction::Impose => "impose",
        MeasureCaptureAction::Confirm => "confirm",
        MeasureCaptureAction::Modify => "modify",
        MeasureCaptureAction::Revoke => "revoke",
        MeasureCaptureAction::Cease => "cease",
        MeasureCaptureAction::SubstituteOut => "substitute_out",
        MeasureCaptureAction::SubstituteIn => "substitute_in",
    }
}
pub(super) fn insert(
    tx: &mut Transaction<'_>,
    operation: &MeasureAdministrativeStoredOperation,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let capture = &operation.capture;
    let review = &capture.review;
    let command = &review.command;
    if measure_administrative_origin_with_decision_history(
        hasher,
        capture,
        &operation.record_history,
    )? != operation.origin
        || capture.records.len() != 1
    {
        return Err(inconsistent(
            "administrative owner differs from its exact evidence",
        ));
    }
    let event = crate::audit_postgres::append_transaction(
        tx,
        &review.actor.email,
        "measure_administrative.recorded",
        &marker(capture),
        capture.recorded_at,
    )?;
    let sequence = i64::try_from(event.event.sequence).map_err(inconsistent)?;
    tx.execute("INSERT INTO case_measure_operations(operation_id,case_id,family,owner_digest,audit_sequence)
        VALUES($1,$2,'a1',$3,$4)",&[&command.operation_id.as_uuid(),&review.case_id.as_uuid(),
        &capture.capture_digest.as_bytes().as_slice(),&sequence]).map_err(port)?;
    let (canonical, view, hash) = match &command.action {
        MeasureAdministrativeAction::Correct(value) => {
            let bytes = value.canonical_bytes();
            let hash = hasher.hash_bytes(&bytes).as_bytes().to_vec();
            (
                Some(bytes),
                Some(crate::measure_correction_codec::view(value)),
                Some(hash),
            )
        }
        MeasureAdministrativeAction::MarkEnteredInError => (None, None, None),
    };
    tx.execute("INSERT INTO case_measure_administrations(operation_id,case_id,action,target_measure_id,
        target_revision,target_capture_digest,reason,correction_canonical,correction_view,correction_digest,
        observed_administration_revision,observed_stage_revision,observed_context_digest,support_format,support_policy,
        recorded_by,recorded_by_email,recorded_by_role,recorded_at_seconds,recorded_at_nanoseconds,
        submission_digest,review_digest,capture_digest)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23)",
        &[&command.operation_id.as_uuid(),&review.case_id.as_uuid(),&action(&command.action),&command.target.id().as_uuid(),
        &i64::from(command.target.revision().get()),&command.target.digest().as_bytes().as_slice(),&command.reason.as_str(),
        &canonical,&view,&hash,&i64::from(command.context.administration_revision.get()),&i64::from(command.context.stage_revision.get()),
        &command.context.context_digest.as_bytes().as_slice(),&review.support.format.as_str(),&review.support.policy.as_str(),
        &review.actor.id.as_uuid(),&review.actor.email,&review.actor.role.as_str(),&capture.recorded_at.unix_timestamp(),
        &(capture.recorded_at.nanosecond() as i32),&review.submission_digest.as_bytes().as_slice(),
        &review.review_digest.as_bytes().as_slice(),&capture.capture_digest.as_bytes().as_slice()]).map_err(port)?;
    let record = &capture.records[0];
    let result = &record.result;
    let subject = result.values.subject();
    let (supervisor_id, supervisor_revision) = match result.values.supervision() {
        MeasureSupervision::Known { participant, .. } => (
            Some(participant.id().as_uuid()),
            Some(i64::from(participant.revision().get())),
        ),
        MeasureSupervision::Unknown { .. } => (None, None),
    };
    let bytes = result.values.canonical_bytes();
    let digest = hasher.hash_bytes(&bytes);
    tx.execute("INSERT INTO case_measure_revisions(measure_id,revision,case_id,owner_operation,family,action,
        values_canonical,values_view,values_digest,capture_digest,subject_id,subject_revision,subject_values_digest,
        supervisor_id,supervisor_revision,validity)
        VALUES($1,$2,$3,$4,'c1',$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
        &[&result.id.as_uuid(),&i64::from(result.revision.get()),&review.case_id.as_uuid(),&command.operation_id.as_uuid(),
        &retained_action(result.last_action),&bytes,&crate::measure_decision_codec::measure_view(&result.values),
        &digest.as_bytes().as_slice(),&record.capture_digest.as_bytes().as_slice(),&subject.id.as_uuid(),
        &i64::from(subject.revision.get()),&subject.values_digest.as_bytes().as_slice(),&supervisor_id,&supervisor_revision,
        &validity(result.validity)]).map_err(port)?;
    Ok(())
}
