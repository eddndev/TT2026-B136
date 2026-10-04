use crate::{decision_support::*, measure_history_support::*};
use time::Duration;

fn joined_history() -> (MeasureDecisionGroupCapture, MeasureHistoryEvidence) {
    let first = root_fixture(1, 70).capture();
    let second = root_fixture(2, 80).capture();
    let mut evidence = history(vec![entry(&first, &empty()), entry(&second, &empty())]);
    let join = confirmation(
        3,
        &[member(&first, 70), member(&second, 80)],
        &evidence,
        at() + Duration::seconds(1),
    );
    append(&mut evidence, &join);
    let final_group = confirmation(
        4,
        &[member(&join, 70)],
        &evidence,
        at() + Duration::seconds(2),
    );
    append(&mut evidence, &final_group);
    (final_group, evidence)
}

#[test]
fn complete_confirm_history_retains_initial_origin_and_unknown_declarations() {
    let (group, evidence) = three_revisions();
    let selected = reference(&group.measures[0]);
    let actual =
        resolve_measure_targets(&Hasher, group.review.case_id, &[selected], &evidence).unwrap();
    assert_eq!(actual.targets(), &[member(&group, 70)]);
    let result = &actual.targets()[0].capture.result;
    let initial = &evidence.groups[0].capture.measures[0].result;
    assert_eq!(result.revision.get(), 3);
    assert_eq!(result.origin, initial.origin);
    assert_eq!(result.values, initial.values);
    assert_eq!(result.sources, initial.sources);
    assert_eq!(
        result
            .values
            .validity()
            .start()
            .unknown_reason()
            .unwrap()
            .as_str(),
        "Start not stated"
    );
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
}

#[test]
fn dependency_order_is_irrelevant_and_targets_are_normalized_by_identity() {
    let first = root_fixture(1, 70).capture();
    let second = root_fixture(2, 80).capture();
    let mut evidence = history(vec![entry(&first, &empty()), entry(&second, &empty())]);
    let selected = [
        reference(&second.measures[0]),
        reference(&first.measures[0]),
    ];
    let expected = resolve_measure_targets(&Hasher, first.review.case_id, &selected, &evidence)
        .unwrap()
        .targets()
        .to_vec();
    evidence.groups.reverse();
    let actual =
        resolve_measure_targets(&Hasher, first.review.case_id, &selected, &evidence).unwrap();
    assert_eq!(actual.targets(), expected);
    assert_eq!(actual.targets(), &[member(&first, 70), member(&second, 80)]);
    let (group, mut evidence) = three_revisions();
    evidence.groups.reverse();
    assert!(resolve_measure_targets(
        &Hasher,
        group.review.case_id,
        &[reference(&group.measures[0])],
        &evidence
    )
    .is_ok());
}

#[test]
fn missing_owning_group_or_any_ancestor_rejects() {
    let (group, evidence) = three_revisions();
    for omitted in 0..evidence.groups.len() {
        let mut incomplete = evidence.clone();
        incomplete.groups.remove(omitted);
        rejects(&group, &incomplete);
    }
    rejects(&group, &empty());
}

#[test]
fn selected_history_requires_the_dependencies_of_unselected_siblings() {
    let (group, evidence) = joined_history();
    let selected = [reference(&group.measures[0])];
    assert!(resolve_measure_targets(&Hasher, group.review.case_id, &selected, &evidence).is_ok());
    let mut missing_sibling_root = evidence.clone();
    missing_sibling_root.groups.remove(1);
    rejects(&group, &missing_sibling_root);
}

#[test]
fn sibling_dependencies_are_validated_beyond_presence_and_origin_digests() {
    let (group, mut evidence) = joined_history();
    let sibling_root = &mut evidence.groups[1].capture;
    sibling_root.measures[0].result.origin.operation_id = group.review.command.operation_id;
    refresh_digests(sibling_root);
    evidence.groups[1] = claimed_entry(sibling_root.clone());
    rejects(&group, &evidence);
}

#[test]
fn unrelated_and_orphan_groups_are_rejected_even_when_they_are_valid() {
    let (group, mut evidence) = three_revisions();
    let unrelated = root_fixture(9, 90).capture();
    evidence.groups.push(entry(&unrelated, &empty()));
    rejects(&group, &evidence);
    evidence.groups.pop();
    let mut fixture = Fixture::no_change();
    fixture.command.operation_id = unrelated.review.command.operation_id;
    fixture.command.decision_id = unrelated.review.command.decision_id;
    let no_change = fixture.capture();
    evidence.groups.push(entry(&no_change, &empty()));
    rejects(&group, &evidence);
}

#[test]
fn duplicate_operation_decision_or_measure_revision_keys_are_rejected() {
    let first = root_fixture(1, 70).capture();
    for mutation in 0..4 {
        let mut fixture = root_fixture(2, if mutation == 2 { 70 } else { 80 });
        match mutation {
            0 => fixture.command.operation_id = first.review.command.operation_id,
            1 => fixture.command.decision_id = first.review.command.decision_id,
            _ => {}
        }
        let second = if mutation == 3 {
            first.clone()
        } else {
            fixture.capture()
        };
        let evidence = history(vec![entry(&first, &empty()), entry(&second, &empty())]);
        let mut selected = vec![reference(&first.measures[0])];
        if second.measures[0].result.id != first.measures[0].result.id {
            selected.push(reference(&second.measures[0]));
        }
        assert!(
            resolve_measure_targets(&Hasher, first.review.case_id, &selected, &evidence).is_err()
        );
    }
}

#[test]
fn a_valid_prefix_is_historical_evidence_without_claiming_the_current_head() {
    let (_, complete) = three_revisions();
    let first = &complete.groups[0].capture;
    let prefix = history(vec![complete.groups[0].clone()]);
    let selected = [reference(&first.measures[0])];
    let resolved =
        resolve_measure_targets(&Hasher, first.review.case_id, &selected, &prefix).unwrap();
    assert_eq!(resolved.targets(), &[member(first, 70)]);
    assert!(resolve_measure_targets(&Hasher, first.review.case_id, &selected, &complete).is_err());
}

#[test]
fn origin_creation_requires_exact_ancestors_and_excludes_the_candidate() {
    let (group, complete) = three_revisions();
    assert!(measure_group_origin(&Hasher, &group, &complete).is_err());
    let mut ancestors = complete.clone();
    ancestors.groups.pop();
    assert!(measure_group_origin(&Hasher, &group, &ancestors).is_ok());
    ancestors.groups.remove(0);
    assert!(measure_group_origin(&Hasher, &group, &ancestors).is_err());
    let first = &complete.groups[0].capture;
    assert!(measure_group_origin(&Hasher, first, &complete).is_err());
}

#[test]
fn self_references_and_cycles_in_owning_group_edges_are_rejected() {
    let (group, evidence) = three_revisions();
    for self_cycle in [false, true] {
        let mut cyclic = evidence.clone();
        let target_owner = if self_cycle {
            member(&cyclic.groups[1].capture, 70).owner
        } else {
            member(&cyclic.groups[2].capture, 70).owner
        };
        cyclic.groups[1].capture.review.material.predecessors[0].owner = target_owner;
        refresh_digests(&mut cyclic.groups[1].capture);
        cyclic.groups[1] = claimed_entry(cyclic.groups[1].capture.clone());
        rejects(&group, &cyclic);
    }
}
