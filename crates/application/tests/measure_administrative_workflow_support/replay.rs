use super::*;
use domain::identity::Role;

#[test]
fn exact_original_replay_preserves_historical_capture_and_skips_document_admission() {
    for fixture in [
        Fixture::single(),
        Fixture::mark(),
        Fixture::after_c(),
        Fixture::after_m2(),
    ] {
        let operation = fixture.operation(at() + Duration::seconds(20));
        let review = operation.capture.review.clone();
        let harness = harness(
            fixture.replay_store(operation.clone()),
            identity(fixture.actor.clone()),
        );
        let actual = harness
            .service
            .prepare("session", fixture.case_id, fixture.command.clone())
            .unwrap();
        assert_eq!(actual, review);
        assert_eq!(harness.validator.calls(), 0);
        assert!(harness.events().is_empty());

        let harness = super::harness(
            fixture.replay_store(operation.clone()),
            identity(fixture.actor),
        );
        let actual = harness
            .service
            .submit(
                "session",
                fixture.case_id,
                fixture.command,
                confirmation(&review),
            )
            .unwrap();
        assert_eq!(actual, operation);
        assert_eq!(harness.validator.calls(), 0);
        assert!(harness.events().is_empty());
    }
}

#[test]
fn replay_uses_original_recording_profile_after_current_profile_changes() {
    let mut fixture = Fixture::single();
    let operation = fixture.operation(at() + Duration::seconds(1));
    let review = operation.capture.review.clone();
    fixture.actor.role = Role::Owner;
    fixture.actor.email = "updated-owner@example.test".into();
    assert_ne!(fixture.actor, review.actor);
    let harness = harness(
        fixture.replay_store(operation.clone()),
        identity(fixture.actor),
    );
    let actual = harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .unwrap();
    assert_eq!(actual, operation);
    assert_eq!(actual.capture.review.actor, review.actor);
    assert!(harness.events().is_empty());
}

#[test]
fn exact_raced_replay_can_retain_a_capture_time_before_the_fresh_commit_floor() {
    let fixture = Fixture::single();
    let operation = fixture.operation(at() + Duration::seconds(1));
    let expected = operation.clone();
    let review = fixture.review();
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, prepared| {
            assert_eq!(prepared.review(), &operation.capture.review);
            Ok(operation)
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
    assert!(actual.capture.recorded_at < now());
    assert_eq!(harness.validator.calls(), 1);
}
