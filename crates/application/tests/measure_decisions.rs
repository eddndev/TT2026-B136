#[path = "measure_decision_fixtures/anchors.rs"]
mod anchor_tests;
#[path = "measure_decision_fixtures/capture_validation.rs"]
mod capture_validation_tests;
#[allow(dead_code)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[path = "measure_decision_fixtures/inventory.rs"]
mod inventory_tests;
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
#[path = "measure_source_support/mod.rs"]
mod measure_source_support;
#[allow(dead_code)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code)]
#[path = "precautionary_receipt_support/mod.rs"]
mod precautionary_receipt_support;
#[path = "measure_decision_fixtures/preparation.rs"]
mod preparation_tests;

use domain::crypto::DocumentHasher;
use domain::identity::Role;
use measure_decision_fixtures::*;

#[test]
fn standalone_imposition_derives_real_decision_measure_and_group_captures() {
    let fixture = Fixture::single();
    let expected = fixture.clone();
    let checked = fixture.prepare().unwrap();
    let review = checked.review();
    assert_eq!(review.case_id, expected.case_id);
    assert_eq!(review.actor, expected.actor);
    assert_eq!(review.command, expected.command);
    assert_eq!(review.material, expected.material);
    assert_eq!(review.results.len(), 1);
    let result = &review.results[0];
    let MeasureEffect::Impose(proposal) = &expected.command.outcome.changes().unwrap()[0] else {
        panic!("imposition fixture expected")
    };
    assert_eq!(result.id, proposal.id);
    assert_eq!(result.revision, MeasureRevision::initial());
    assert_eq!(result.origin.decision_id, expected.command.decision_id);
    assert_eq!(result.origin.operation_id, expected.command.operation_id);
    assert_eq!(result.effect_key, result.id);
    assert_eq!(result.action, MeasureCaptureAction::Impose);
    assert_eq!(result.previous, None);
    assert_eq!(result.values, proposal.values);
    assert_eq!(result.sources, expected.material.result_sources[0].sources);
    assert_eq!(
        result.projection,
        resolve_measure_sources(&Hasher, expected.case_id, &result.values, &result.sources,)
            .unwrap()
    );
    let review = review.clone();
    let group = checked.into_group_capture(&Hasher, at()).unwrap();
    assert_eq!(group.review, review);
    assert_eq!(group.measures.len(), 1);
    assert!(group.substitutions.is_empty());
    let decision = &group.decision;
    assert_eq!(decision.case_id, expected.case_id);
    assert_eq!(decision.operation_id, expected.command.operation_id);
    assert_eq!(decision.decision_id, expected.command.decision_id);
    assert_eq!(decision.actor, expected.actor);
    assert_eq!(decision.context, expected.material.context);
    assert_eq!(decision.values, expected.command.values);
    assert_eq!(decision.support, expected.material.support);
    assert_eq!(decision.anchor, None);
    let measure = &group.measures[0];
    assert_eq!(measure.case_id, expected.case_id);
    assert_eq!(measure.result, review.results[0]);
    assert_eq!(measure.operation_id, decision.operation_id);
    assert_eq!(measure.decision_id, decision.decision_id);
    assert_eq!(measure.decision_digest, decision.capture_digest);
    assert_eq!(measure.actor, expected.actor);
    assert_eq!(
        (decision.recorded_at, measure.recorded_at, group.recorded_at),
        (at(), at(), at())
    );
    assert_eq!(
        group.capture_digest,
        Hasher.hash_bytes(&measure_decision_group_bytes(&group).unwrap())
    );
    measure_decision_group_matches(&Hasher, &group).unwrap();
}

#[test]
fn no_measure_change_still_records_the_decision_and_complete_empty_group() {
    let fixture = Fixture::no_change();
    let values = fixture.command.values.clone();
    let group = fixture.capture();
    assert!(group.review.results.is_empty());
    assert!(group.measures.is_empty());
    assert!(group.substitutions.is_empty());
    assert_eq!(group.decision.values, values);
    assert_eq!(
        group
            .review
            .command
            .outcome
            .no_measure_change()
            .unwrap()
            .as_str(),
        "No measure change stated"
    );
    measure_decision_group_matches(&Hasher, &group).unwrap();
}

#[test]
fn source_order_is_normalized_before_review_and_group_hashing() {
    let fixture = Fixture::multiple();
    let expected = fixture.clone().capture();
    let mut reversed = fixture;
    reversed.material.result_sources.reverse();
    let actual = reversed.capture();
    assert_eq!(actual, expected);
    assert_eq!(
        actual
            .measures
            .iter()
            .map(|item| item.result.id)
            .collect::<Vec<_>>(),
        vec![id(70), id(80)]
    );
}

#[test]
fn thirty_two_distinct_impositions_retain_their_complete_source_inventory() {
    let mut fixture = Fixture::single();
    let source = fixture.material.result_sources[0].sources.clone();
    let MeasureEffect::Impose(proposal) = &fixture.command.outcome.changes().unwrap()[0] else {
        panic!("imposition fixture expected")
    };
    let values = proposal.values.clone();
    for index in 71..102 {
        fixture.add_imposition(index, values.clone(), source.clone());
    }
    let group = fixture.capture();
    assert_eq!(group.measures.len(), 32);
    assert_eq!(group.review.results.len(), 32);
    for member in &group.measures {
        assert_eq!(member.result.sources, source);
        assert_eq!(member.result.revision, MeasureRevision::initial());
    }
    measure_decision_group_matches(&Hasher, &group).unwrap();
}

#[test]
fn instructions_bind_all_roles_and_captures_preserve_permitted_recording_roles() {
    let mut instructions = Vec::new();
    for role in [Role::Owner, Role::Litigator, Role::Paralegal, Role::Client] {
        let mut fixture = Fixture::single();
        fixture.actor.role = role;
        instructions.push(
            measure_decision_submission_bytes(&fixture.actor, fixture.case_id, &fixture.command)
                .unwrap(),
        );
        if matches!(role, Role::Paralegal | Role::Client) {
            assert!(fixture.prepare().is_err());
            continue;
        }
        let group = fixture.capture();
        assert_eq!(group.review.actor.role, role);
        assert_eq!(group.decision.actor.role, role);
        assert_eq!(group.measures[0].actor.role, role);
        measure_decision_group_matches(&Hasher, &group).unwrap();
    }
    for index in 0..instructions.len() {
        for other in &instructions[index + 1..] {
            assert_ne!(&instructions[index], other);
        }
    }
}

#[test]
fn capture_clock_does_not_replace_unknown_declared_decision_start_or_validity() {
    let group = Fixture::single().capture();
    assert_eq!(
        group
            .decision
            .values
            .declared_at()
            .unknown_reason()
            .unwrap()
            .as_str(),
        "Decision time not stated"
    );
    let values = &group.measures[0].result.values;
    assert_eq!(
        values.validity().start().unknown_reason().unwrap().as_str(),
        "Start not stated"
    );
    assert!(values.validity().end().is_none());
    assert_eq!(group.recorded_at, at());
}
