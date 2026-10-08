use super::{inconsistent, port};
use application::{
    precautionary_hearings::{
        PrecautionaryHearingAction, PrecautionaryHearingCapture, PrecautionaryHearingChange,
    },
    ApplicationError,
};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(super) fn marker(capture: &PrecautionaryHearingCapture) -> String {
    let review = &capture.review;
    format!(
        "ph1:case:{}:hearing:{}:operation:{}:revision:{}:submission:{}:review:{}:capture:{}",
        review.case_id,
        review.command.hearing_id,
        review.command.operation_id,
        review.result_revision.get(),
        review.submission_digest.to_hex(),
        review.review_digest.to_hex(),
        capture.capture_digest.to_hex(),
    )
}

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    capture: &PrecautionaryHearingCapture,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let review = &capture.review;
    let command = &review.command;
    let (values, previous, reason) = match &command.change {
        PrecautionaryHearingChange::Schedule { values, .. } => (Some(values), None, None),
        PrecautionaryHearingChange::Replace {
            values,
            expected_capture_digest,
            reason,
            ..
        } => (
            Some(values),
            Some(expected_capture_digest.as_bytes().as_slice()),
            Some(reason.as_str()),
        ),
        PrecautionaryHearingChange::Cancel {
            expected_capture_digest,
            reason,
            ..
        } => (
            None,
            Some(expected_capture_digest.as_bytes().as_slice()),
            Some(reason.as_str()),
        ),
    };
    let canonical = values.map(|value| value.canonical_bytes());
    let projection = values.map(crate::precautionary_hearing_codec::view);
    let values_digest = canonical
        .as_ref()
        .map(|bytes| hasher.hash_bytes(bytes).as_bytes().to_vec());
    let support_format = values.map(|_| review.sources.support.format.as_str());
    let support_policy = values.map(|_| review.sources.support.policy.as_str());
    let observed = review.observed_context.material();
    let context_digest = review.observed_context.digest(hasher);
    if command.action() == PrecautionaryHearingAction::Schedule {
        tx.execute(
            "INSERT INTO case_precautionary_hearings(id,case_id) VALUES($1,$2)",
            &[&command.hearing_id.as_uuid(), &review.case_id.as_uuid()],
        )
        .map_err(port)?;
    }
    let action = format!("precautionary_hearing.{}", command.action().as_str());
    let entry = crate::audit_postgres::append_transaction(
        tx,
        &review.actor.email,
        &action,
        &marker(capture),
        capture.recorded_at,
    )?;
    let audit_sequence = i64::try_from(entry.event.sequence)
        .map_err(|_| inconsistent("precautionary audit sequence exceeds storage range"))?;
    tx.execute(
        "INSERT INTO case_precautionary_hearing_revisions(
            hearing_id,case_id,revision,operation_id,action,previous_capture_digest,reason,
            values_canonical,values_view,values_digest,observed_administration_revision,
            observed_stage_revision,observed_context_digest,support_format,support_policy,
            recorded_by,recorded_by_email,recorded_by_role,recorded_at_seconds,
            recorded_at_nanoseconds,submission_digest,review_digest,capture_digest,audit_sequence)
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
            $21,$22,$23,$24)",
        &[
            &command.hearing_id.as_uuid(),
            &review.case_id.as_uuid(),
            &i64::from(review.result_revision.get()),
            &command.operation_id.as_uuid(),
            &command.action().as_str(),
            &previous,
            &reason,
            &canonical,
            &projection,
            &values_digest,
            &i64::from(observed.administration.revision.get()),
            &i64::from(observed.stage.stage_revision().get()),
            &context_digest.as_bytes().as_slice(),
            &support_format,
            &support_policy,
            &review.actor.id.as_uuid(),
            &review.actor.email,
            &review.actor.role.as_str(),
            &capture.recorded_at.unix_timestamp(),
            &(capture.recorded_at.nanosecond() as i32),
            &review.submission_digest.as_bytes().as_slice(),
            &review.review_digest.as_bytes().as_slice(),
            &capture.capture_digest.as_bytes().as_slice(),
            &audit_sequence,
        ],
    )
    .map_err(port)?;
    crate::alerts_postgres::invalidate(
        tx,
        application::alerts::AlertSubject::PrecautionaryHearing {
            case_id: review.case_id,
            id: command.hearing_id,
        },
        hasher,
    )?;
    Ok(())
}
