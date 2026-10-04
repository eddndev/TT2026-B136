use super::*;
use application::precautionary_measures::MeasureDecisionAnchorRef;
use application::precautionary_measures::{
    measure_group_origin, prepare_measure_decision_with_history, MeasureDecisionAnchorMaterial,
    MeasureGroupEvidence,
};
use domain::crypto::DocumentHasher;

#[test]
fn regression_prepare_rejects_full_capture_disagreement_between_prefix_and_measure_anchor() {
    let original = Fixture::schedule().operation(at());
    let mut altered_anchor = original.capture.clone();
    altered_anchor.recorded_at += Duration::seconds(1);
    altered_anchor.capture_digest =
        Hasher.hash_bytes(&precautionary_hearing_capture_bytes(&altered_anchor).unwrap());
    precautionary_hearing_receipt_matches(&Hasher, &altered_anchor).unwrap();
    assert_eq!(altered_anchor.review, original.capture.review);

    let mut decision = crate::measure_decision_fixtures::Fixture::single();
    decision.material.context = altered_anchor.review.observed_context.clone();
    decision.command.context = crate::receipt_support::expectation(&decision.material.context);
    decision.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: altered_anchor.review.command.hearing_id,
        revision: altered_anchor.review.result_revision,
        capture_digest: altered_anchor.capture_digest,
    });
    decision.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        altered_anchor,
    )));
    let group = prepare_measure_decision_with_history(
        &Hasher,
        &decision.actor,
        decision.case_id,
        decision.command,
        decision.material,
        &empty_history(),
    )
    .unwrap()
    .into_group_capture(&Hasher, at() + Duration::seconds(2))
    .unwrap();
    let origin = measure_group_origin(&Hasher, &group, &empty_history()).unwrap();
    let mut fixture = Fixture::replace(&original);
    let PrecautionaryHearingChange::Replace { values, .. } = &mut fixture.command.change else {
        unreachable!()
    };
    let mut input = values_input(values);
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = vec![crate::measure_decision_fixtures::reference(
        &group.measures[0],
    )];
    *values = PrecautionaryHearingValues::new(input).unwrap();
    fixture
        .material
        .measure_history
        .groups
        .push(MeasureGroupEvidence {
            origin,
            capture: group,
        });

    let harness = harness(fixture.store(), identity(fixture.actor.clone()));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}
