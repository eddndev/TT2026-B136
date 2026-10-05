use super::{anchors, inconsistent, port};
use application::{
    precautionary_measures::{
        measure_group_origin, MeasureDecisionGroupCapture, MeasureDecisionStoredOperation,
    },
    ApplicationError,
};
use domain::{crypto::DocumentHasher, precautionary_measures::MeasureSupervision};
use postgres::Transaction;

pub(super) fn marker(group: &MeasureDecisionGroupCapture) -> String {
    let review = &group.review;
    format!(
        "mg1:case:{}:operation:{}:decision:{}:submission:{}:review:{}:decision_digest:{}:group:{}",
        review.case_id,
        review.command.operation_id,
        review.command.decision_id,
        review.submission_digest.to_hex(),
        review.review_digest.to_hex(),
        group.decision.capture_digest.to_hex(),
        group.capture_digest.to_hex(),
    )
}

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    operation: &MeasureDecisionStoredOperation,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let group = &operation.group;
    let review = &group.review;
    let command = &review.command;
    if measure_group_origin(hasher, group, &operation.measure_history)? != operation.origin {
        return Err(inconsistent(
            "measure group differs from its original evidence",
        ));
    }
    let entry = crate::audit_postgres::append_transaction(
        tx,
        &review.actor.email,
        "measure_decision.recorded",
        &marker(group),
        group.recorded_at,
    )?;
    let sequence = i64::try_from(entry.event.sequence)
        .map_err(|_| inconsistent("measure audit sequence exceeds storage range"))?;
    tx.execute(
        "INSERT INTO case_measure_operations(operation_id,case_id,family,owner_digest,audit_sequence)
         VALUES($1,$2,'g1',$3,$4)",
        &[&command.operation_id.as_uuid(), &review.case_id.as_uuid(),
            &group.capture_digest.as_bytes().as_slice(), &sequence],
    ).map_err(port)?;
    let values = command.values.canonical_bytes();
    let values_view = crate::measure_decision_codec::decision_view(&command.values);
    let values_digest = hasher.hash_bytes(&values);
    let outcome = command.outcome.canonical_bytes();
    let outcome_view = crate::measure_decision_codec::outcome_view(&command.outcome);
    let outcome_digest = hasher.hash_bytes(&outcome);
    let context = &review.material.context;
    let context_digest = context.digest(hasher);
    let support = &review.material.support;
    let anchor = anchors::columns(&command.anchor)?;
    tx.execute(
        "INSERT INTO case_measure_decisions(decision_id,operation_id,case_id,
            values_canonical,values_view,values_digest,outcome_canonical,outcome_view,outcome_digest,
            observed_administration_revision,observed_stage_revision,observed_context_digest,
            support_format,support_policy,recorded_by,recorded_by_email,recorded_by_role,
            recorded_at_seconds,recorded_at_nanoseconds,submission_digest,review_digest,
            decision_digest,group_digest,anchor_kind,anchor_hearing_id,anchor_revision,
            anchor_values_digest,anchor_submission_digest,anchor_precautionary_hearing_id,anchor_capture_digest)
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30)",
        &[
            &command.decision_id.as_uuid(), &command.operation_id.as_uuid(), &review.case_id.as_uuid(),
            &values, &values_view, &values_digest.as_bytes().as_slice(),
            &outcome, &outcome_view, &outcome_digest.as_bytes().as_slice(),
            &i64::from(context.material().administration.revision.get()),
            &i64::from(context.material().stage.stage_revision().get()),
            &context_digest.as_bytes().as_slice(), &support.format.as_str(), &support.policy.as_str(),
            &review.actor.id.as_uuid(), &review.actor.email, &review.actor.role.as_str(),
            &group.recorded_at.unix_timestamp(), &(group.recorded_at.nanosecond() as i32),
            &review.submission_digest.as_bytes().as_slice(), &review.review_digest.as_bytes().as_slice(),
            &group.decision.capture_digest.as_bytes().as_slice(), &group.capture_digest.as_bytes().as_slice(),
            &anchor.kind, &anchor.hearing_id, &anchor.revision,
            &anchor.values_digest, &anchor.submission_digest, &anchor.precautionary_hearing_id, &anchor.capture_digest,
        ],
    ).map_err(port)?;
    for capture in &group.measures {
        let result = &capture.result;
        let subject = result.values.subject();
        let (supervisor_id, supervisor_revision) = match result.values.supervision() {
            MeasureSupervision::Known { participant, .. } => (
                Some(participant.id().as_uuid()),
                Some(i64::from(participant.revision().get())),
            ),
            MeasureSupervision::Unknown { .. } => (None, None),
        };
        let canonical = result.values.canonical_bytes();
        let projection = crate::measure_decision_codec::measure_view(&result.values);
        let digest = hasher.hash_bytes(&canonical);
        if result.previous.is_none() {
            tx.execute(
                "INSERT INTO case_measures(id,case_id,root_operation) VALUES($1,$2,$3)",
                &[
                    &result.id.as_uuid(),
                    &review.case_id.as_uuid(),
                    &result.origin.operation_id.as_uuid(),
                ],
            )
            .map_err(port)?;
        }
        tx.execute(
            "INSERT INTO case_measure_revisions(measure_id,revision,case_id,owner_operation,
                family,action,values_canonical,values_view,values_digest,capture_digest,
                subject_id,subject_revision,subject_values_digest,supervisor_id,supervisor_revision)
             VALUES($1,$2,$3,$4,'m1',$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
            &[
                &result.id.as_uuid(),
                &i64::from(result.revision.get()),
                &review.case_id.as_uuid(),
                &command.operation_id.as_uuid(),
                &super::decode::action_name(result.action),
                &canonical,
                &projection,
                &digest.as_bytes().as_slice(),
                &capture.capture_digest.as_bytes().as_slice(),
                &subject.id.as_uuid(),
                &i64::from(subject.revision.get()),
                &subject.values_digest.as_bytes().as_slice(),
                &supervisor_id,
                &supervisor_revision,
            ],
        )
        .map_err(port)?;
    }
    Ok(())
}
