use super::*;
use domain::identity::Role;

fn submit(fixture: Fixture) -> MeasureAdministrativeStoredOperation {
    let expected = fixture.operation(now());
    let review = fixture.review();
    let actor = fixture.actor.clone();
    let material = fixture.material.clone();
    let expected_review = review.clone();
    let case_id = fixture.case_id;
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |actual, case, prepared| {
            assert_eq!(actual, &actor);
            assert_eq!(case, case_id);
            assert_eq!(prepared.actor(), &actor);
            assert_eq!(prepared.material(), &material);
            assert_eq!(prepared.review(), &expected_review);
            prepared.into_operation(now())
        });
    let harness = harness(store, identity(fixture.actor));
    let actual = harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.capture.records.len(), 1);
    assert_eq!(harness.validator.calls(), 1);
    actual
}

#[test]
fn confirmed_corrections_commit_exact_rows_after_judicial_and_administrative_targets() {
    for fixture in [
        Fixture::single(),
        Fixture::after_c(),
        Fixture::after_m2(),
        Fixture::after_corrected_m2(),
    ] {
        let target = fixture.command.target;
        let result = submit(fixture);
        assert_eq!(result.capture.records[0].result.previous, target);
        assert_eq!(
            result.capture.records[0].result.validity,
            MeasureCaptureValidity::Valid
        );
    }
}

#[test]
fn mark_commits_the_original_judicial_values_and_action_with_invalid_capture_status() {
    let fixture = Fixture::mark();
    let previous = fixture.history.records.judicial.groups[0].capture.measures[0].clone();
    let actual = submit(fixture);
    let result = &actual.capture.records[0].result;
    assert_eq!(result.validity, MeasureCaptureValidity::EnteredInError);
    assert_eq!(result.values, previous.result.values);
    assert_eq!(result.sources, previous.result.sources);
    assert_eq!(result.last_action, previous.result.action);
    assert_eq!(result.last_judicial.reference, reference(&previous));
}

#[test]
fn owner_and_litigator_record_their_complete_current_principal() {
    for role in [Role::Owner, Role::Litigator] {
        let mut fixture = Fixture::single();
        fixture.actor.role = role;
        fixture.actor.email = "current-recorder@example.test".into();
        let expected_actor = fixture.actor.clone();
        let actual = submit(fixture);
        assert_eq!(actual.capture.review.actor, expected_actor);
        assert_eq!(actual.capture.records[0].actor, expected_actor);
    }
}

#[test]
fn correction_admits_the_latest_actual_judicial_support_after_modification() {
    let fixture = Fixture::after_modification();
    let initial = &fixture.history.records.judicial.groups[0].capture;
    let latest = fixture.review();
    assert_ne!(latest.support.reference, initial.decision.support.reference);
    assert_eq!(
        latest.support.reference.id,
        fixture.material.support_record.id
    );
    let actual = submit(fixture);
    assert_eq!(actual.capture.review.support, latest.support);
}

#[test]
fn unrelated_valid_forest_owners_are_excluded_from_the_stored_exact_target_closure() {
    let mut fixture = Fixture::single();
    let original = &fixture.history.records.judicial.groups[0].capture;
    let mut independent = crate::measure_dependency_support::independent_request(1, 9001);
    independent.command.values = original.review.command.values.clone();
    independent.material.support = original.decision.support.clone();
    let group = independent.capture();
    let mut forest = fixture
        .material
        .dependency_inventory
        .records
        .records
        .judicial
        .clone();
    forest.groups.extend(
        crate::effect_support::append_history(&crate::effect_support::empty_history(), &group)
            .groups,
    );
    fixture
        .material
        .dependency_inventory
        .records
        .records
        .judicial = forest;
    let expected_history = fixture.history.clone();
    let actual = submit(fixture);
    assert_eq!(actual.record_history, expected_history);
    assert_eq!(actual.record_history.records.judicial.groups.len(), 1);
}
