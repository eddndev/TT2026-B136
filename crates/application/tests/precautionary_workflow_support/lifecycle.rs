use super::*;
use domain::hearings::HearingStatus;
use domain::identity::Role;
use time::Duration;

#[test]
fn owner_and_litigator_can_prepare_using_their_actual_principal() {
    for role in [Role::Owner, Role::Litigator] {
        let mut fixture = Fixture::schedule();
        fixture.actor.role = role;
        let expected = fixture.review();
        let harness = harness(fixture.store(), identity(fixture.actor.clone()));
        let review = harness
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .unwrap();
        assert_eq!(review, expected);
        assert_eq!(review.actor.role, role);
    }
}

#[test]
fn schedule_submit_passes_opaque_reviewed_material_and_returns_exact_operation() {
    let fixture = Fixture::schedule();
    let review = fixture.review();
    let expected = fixture.operation(at() + Duration::seconds(100));
    let mut store = fixture.store();
    let actor = fixture.actor.clone();
    let case_id = fixture.case_id;
    let material = fixture.material.clone();
    let reviewed = review.clone();
    store
        .expect_commit()
        .times(1)
        .return_once(move |actual, case, prepared| {
            assert_eq!(actual, &actor);
            assert_eq!(case, case_id);
            assert_eq!(prepared.actor(), &actor);
            assert_eq!(prepared.material(), &material);
            assert_eq!(prepared.review(), &reviewed);
            prepared.into_operation(at() + Duration::seconds(100))
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
    assert_eq!(operation.history.captures.len(), 1);
}

#[test]
fn replacement_preserves_the_initial_origin_and_exact_full_predecessor_chain() {
    let initial = Fixture::schedule().operation(at());
    let fixture = Fixture::replace(&initial);
    let review = fixture.review();
    let capture_at = at() + Duration::seconds(100);
    let expected = fixture.operation(capture_at);
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, prepared| prepared.into_operation(capture_at));
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
    assert_eq!(operation.history.origin, initial.history.origin);
    assert_eq!(operation.history.captures[0], initial.capture);
    assert_eq!(operation.capture.review.result_revision.get(), 2);
    assert_eq!(operation.capture.review.status, HearingStatus::Scheduled);
}

#[test]
fn cancellation_keeps_prior_values_sources_and_scheduling_context() {
    let initial = Fixture::schedule().operation(at());
    let replaced = Fixture::replace(&initial).operation(at() + Duration::seconds(2));
    let fixture = Fixture::cancel(&replaced);
    let review = fixture.review();
    let capture_at = at() + Duration::seconds(100);
    let expected = fixture.operation(capture_at);
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, prepared| prepared.into_operation(capture_at));
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
    let cancelled = &operation.capture.review;
    assert_eq!(cancelled.status, HearingStatus::Cancelled);
    assert_eq!(
        cancelled.resolved_values,
        replaced.capture.review.resolved_values
    );
    assert_eq!(cancelled.sources, replaced.capture.review.sources);
    assert_eq!(
        cancelled.scheduling_context,
        replaced.capture.review.scheduling_context
    );
    assert_eq!(operation.history.captures.len(), 3);
}

#[test]
fn exact_replay_returns_the_original_review_and_capture_timestamp() {
    let fixture = Fixture::schedule();
    let original = fixture.operation(at());
    let harness = harness(
        fixture.replay_store(original.clone()),
        identity(fixture.actor.clone()),
    );
    assert_eq!(
        harness
            .service
            .prepare("session", fixture.case_id, fixture.command.clone())
            .unwrap(),
        original.capture.review,
    );
    let harness = super::harness(
        fixture.replay_store(original.clone()),
        identity(fixture.actor),
    );
    let result = harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&original.capture.review),
        )
        .unwrap();
    assert_eq!(result, original);
    assert_eq!(result.capture.recorded_at, at());
}

#[test]
fn raced_exact_operation_returns_its_earlier_capture_without_a_new_timestamp() {
    let fixture = Fixture::schedule();
    let original = fixture.operation(at());
    let mut store = fixture.store();
    let raced = original.clone();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, _| Ok(raced));
    let harness = harness(store, identity(fixture.actor));
    let result = harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&original.capture.review),
        )
        .unwrap();
    assert_eq!(result, original);
}

#[test]
fn replay_uses_historical_actor_metadata_after_an_authorized_profile_change() {
    let original_fixture = Fixture::schedule();
    let original = original_fixture.operation(at());
    let mut request = original_fixture;
    request.actor.email = "current-owner@example.test".into();
    request.actor.role = Role::Owner;
    let harness = harness(
        request.replay_store(original.clone()),
        identity(request.actor.clone()),
    );
    let result = harness
        .service
        .submit(
            "session",
            request.case_id,
            request.command,
            confirmation(&original.capture.review),
        )
        .unwrap();
    assert_eq!(result, original);
    assert_ne!(result.capture.review.actor, request.actor);
    assert_eq!(result.capture.review.actor.role, Role::Litigator);
}
