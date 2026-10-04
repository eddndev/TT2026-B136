use super::*;

#[test]
fn confirm_modify_revoke_and_cease_use_their_exact_owning_history() {
    let initial = Fixture::single().operation(at());
    let cases = [
        (Fixture::confirm(&initial), MeasureCaptureAction::Confirm),
        (Fixture::modify(&initial), MeasureCaptureAction::Modify),
        (Fixture::revoke(&initial), MeasureCaptureAction::Revoke),
        (Fixture::cease(&initial), MeasureCaptureAction::Cease),
    ];
    for (fixture, action) in cases {
        let review = fixture.review();
        let expected = fixture.operation(now());
        let prepared = harness(fixture.store(), identity(fixture.actor.clone()));
        assert_eq!(
            prepared
                .service
                .prepare("session", fixture.case_id, fixture.command.clone())
                .unwrap(),
            review
        );
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
        let measure = &result.group.measures[0].result;
        assert_eq!(measure.action, action);
        assert_eq!(measure.revision.get(), 2);
        assert_eq!(measure.origin, initial.group.measures[0].result.origin);
        assert_eq!(result.measure_history.groups[0].capture, initial.group);
        assert!(measure.values.validity().end().is_none());
    }
}

#[test]
fn many_to_many_substitution_preserves_all_members_and_the_joint_relationship() {
    let initial = Fixture::multiple().operation(at());
    let fixture = Fixture::substitute(&initial);
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
    assert_eq!(result.group.measures.len(), 4);
    let relationship = &result.group.substitutions[0];
    assert_eq!(relationship.predecessors.len(), 2);
    assert_eq!(relationship.successors.len(), 2);
    assert_eq!(
        result
            .group
            .measures
            .iter()
            .filter(|m| m.result.action == MeasureCaptureAction::SubstituteOut)
            .count(),
        2
    );
    assert_eq!(
        result
            .group
            .measures
            .iter()
            .filter(|m| m.result.action == MeasureCaptureAction::SubstituteIn)
            .count(),
        2
    );
}

#[test]
fn normalized_ancestor_order_is_not_a_different_committed_operation() {
    let initial = Fixture::single().operation(at());
    let second = Fixture::confirm(&initial).operation(at() + Duration::seconds(1));
    let mut fixture = Fixture::confirm(&second);
    fixture.material.measure_history.groups.reverse();
    let review = fixture.review();
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| {
            let mut operation = prepared.into_operation(now())?;
            operation.measure_history.groups.reverse();
            Ok(operation)
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
    assert_eq!(operation.group.review, review);
    assert_eq!(operation.measure_history.groups.len(), 2);
    measure_decision_group_with_history_matches(
        &Hasher,
        &operation.group,
        &operation.measure_history,
    )
    .unwrap();
}
