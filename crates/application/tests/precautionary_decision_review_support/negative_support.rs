use super::*;

pub(super) fn refresh_hearing(capture: &mut PrecautionaryHearingCapture) {
    let review = &mut capture.review;
    review.submission_digest = Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &review.actor,
            review.case_id,
            &review.command,
            &review.resolved_values,
        )
        .unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&precautionary_hearing_review_bytes(review).unwrap());
    capture.capture_digest =
        Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture).unwrap());
}

pub(super) fn refresh_group(group: &mut MeasureDecisionGroupCaptureV2) {
    let review = &mut group.review;
    review.submission_digest = Hasher.hash_bytes(
        &measure_decision_submission_bytes(&review.actor, review.case_id, &review.command).unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&measure_decision_review_v2_bytes(review).unwrap());
    group.decision.capture_digest =
        Hasher.hash_bytes(&measure_decision_capture_bytes(&group.decision).unwrap());
    for row in &mut group.measures {
        row.capture_digest = Hasher.hash_bytes(&measure_capture_v2_bytes(row).unwrap());
    }
    group.capture_digest = Hasher.hash_bytes(&measure_decision_group_v2_bytes(group).unwrap());
}

pub(super) fn refresh_administrative(capture: &mut MeasureAdministrativeCapture) {
    let review = &mut capture.review;
    review.submission_digest = Hasher.hash_bytes(
        &measure_administrative_submission_bytes(&review.actor, review.case_id, &review.command)
            .unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&measure_administrative_review_bytes(review).unwrap());
    for row in &mut capture.records {
        row.review_digest = review.review_digest;
        row.capture_digest = Hasher.hash_bytes(&measure_administrative_record_bytes(row).unwrap());
    }
    capture.capture_digest =
        Hasher.hash_bytes(&measure_administrative_capture_bytes(capture).unwrap());
}

pub(super) fn retarget(capture: &mut PrecautionaryHearingCapture, target: PrecautionaryMeasureRef) {
    let mut input = crate::record_review_support::values_input(&capture.review.resolved_values);
    input.review_targets = vec![target];
    let changed = PrecautionaryHearingValues::new(input).unwrap();
    capture.review.resolved_values = changed.clone();
    match &mut capture.review.command.change {
        PrecautionaryHearingChange::Schedule { values, .. }
        | PrecautionaryHearingChange::Replace { values, .. } => *values = changed,
        _ => panic!("scheduled values required"),
    }
    refresh_hearing(capture);
}

pub(super) fn claimed_hearing_origin(
    capture: &PrecautionaryHearingCapture,
) -> PrecautionaryHearingOrigin {
    let review = &capture.review;
    PrecautionaryHearingOrigin {
        case_id: review.case_id,
        hearing_id: review.command.hearing_id,
        operation_id: review.command.operation_id,
        revision: review.result_revision,
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
        capture_digest: capture.capture_digest,
    }
}
