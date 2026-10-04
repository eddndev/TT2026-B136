use super::*;
use application::precautionary_measures::MeasureDecisionAnchorRef;
use application::precautionary_measures::{
    measure_group_origin, prepare_measure_decision_with_history, MeasureDecisionAnchorMaterial,
    MeasureGroupEvidence, MeasureHistoryEvidence,
};

#[test]
fn regression_review_preparation_cannot_reuse_a_dependency_anchor_hearing_operation() {
    let original = operation(20);
    let anchor = original.capture;
    let empty = MeasureHistoryEvidence { groups: vec![] };
    let mut decision = crate::measure_decision_fixtures::Fixture::single();
    decision.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: anchor.review.command.hearing_id,
        revision: anchor.review.result_revision,
        capture_digest: anchor.capture_digest,
    });
    decision.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        anchor.clone(),
    )));
    let group = prepare_measure_decision_with_history(
        &Hasher,
        &decision.actor,
        decision.case_id,
        decision.command,
        decision.material,
        &empty,
    )
    .unwrap()
    .into_group_capture(&Hasher, at() + time::Duration::seconds(1))
    .unwrap();
    let origin = measure_group_origin(&Hasher, &group, &empty).unwrap();
    let mut fixture = crate::receipt_support::Fixture::schedule();
    let mut input = crate::receipt_support::values_input();
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = vec![crate::measure_decision_fixtures::reference(
        &group.measures[0],
    )];
    fixture.command.hearing_id = id(30);
    fixture.command.operation_id = anchor.review.command.operation_id;
    fixture.command.change = PrecautionaryHearingChange::Schedule {
        context: crate::receipt_support::expectation(&fixture.context),
        values: PrecautionaryHearingValues::new(input).unwrap(),
    };
    let evidence = MeasureHistoryEvidence {
        groups: vec![MeasureGroupEvidence {
            origin,
            capture: group,
        }],
    };
    assert!(prepare_precautionary_hearing_with_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        PrecautionaryHearingPreparationMaterial {
            observed_context: fixture.context,
            sources: fixture.sources,
            predecessor: None,
            measure_history: &evidence,
        },
    )
    .is_err());
}
