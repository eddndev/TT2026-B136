use super::*;
use domain::identity::Role;

#[test]
fn owner_and_litigator_prepare_with_their_actual_captured_principal() {
    for role in [Role::Owner, Role::Litigator] {
        let mut fixture = Fixture::single();
        fixture.actor.role = role;
        let expected = fixture.review();
        let harness = harness(fixture.store(), identity(fixture.actor.clone()));
        assert_eq!(
            harness
                .service
                .prepare("session", fixture.case_id, fixture.command)
                .unwrap(),
            expected
        );
    }
}

#[test]
fn submit_passes_complete_opaque_material_and_returns_the_exact_group_and_origin() {
    let fixture = Fixture::multiple();
    let review = fixture.review();
    let expected = fixture.operation(now());
    let actor = fixture.actor.clone();
    let case_id = fixture.case_id;
    let material = fixture.material.clone();
    let reviewed = review.clone();
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |actual, case, prepared| {
            assert_eq!(actual, &actor);
            assert_eq!(case, case_id);
            assert_eq!(prepared.actor(), &actor);
            assert_eq!(prepared.material(), &material);
            assert_eq!(prepared.review(), &reviewed);
            prepared.into_operation(now())
        });
    let harness = harness(store, identity(fixture.actor));
    let operation = harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .unwrap();
    assert_eq!(operation, expected);
    assert_eq!(operation.group.measures.len(), 2);
    assert_eq!(harness.validator.calls(), 1);
}

#[test]
fn no_measure_change_admits_support_and_commits_a_decision_group_without_measure_rows() {
    let fixture = Fixture::no_change();
    let review = fixture.review();
    let expected = fixture.operation(now());
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_operation(now()));
    let harness = harness(store, identity(fixture.actor));
    let operation = harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .unwrap();
    assert_eq!(operation, expected);
    assert!(operation.group.measures.is_empty());
    assert!(operation.group.substitutions.is_empty());
    assert_eq!(harness.validator.calls(), 1);
}

#[test]
fn ordinary_and_precautionary_anchors_are_retained_exactly_by_the_service() {
    for fixture in [Fixture::initial_anchor(), Fixture::precautionary_anchor()] {
        let review = fixture.review();
        let expected = fixture.operation(now());
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(|_, _, prepared| prepared.into_operation(now()));
        let harness = harness(store, identity(fixture.actor));
        let result = harness
            .service
            .submit(
                "session",
                fixture.case_id,
                fixture.command,
                confirmation(&review),
            )
            .unwrap();
        assert_eq!(result, expected);
        assert!(result.group.decision.anchor.is_some());
    }
}

#[test]
fn review_anchor_no_change_keeps_its_measure_dependency_without_inventing_effects() {
    let initial = Fixture::single().operation(at());
    let fixture = Fixture::review_anchor_no_change(&initial);
    let review = fixture.review();
    let expected = fixture.operation(now());
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_operation(now()));
    let harness = harness(store, identity(fixture.actor));
    let result = harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .unwrap();
    assert_eq!(result, expected);
    assert!(result.group.measures.is_empty());
    assert_eq!(result.measure_history.groups.len(), 1);
    assert_eq!(result.measure_history.groups[0].capture, initial.group);
}
