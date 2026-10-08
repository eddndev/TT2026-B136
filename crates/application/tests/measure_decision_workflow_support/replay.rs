use super::*;
use domain::crypto::Sha256Digest;
use domain::hearings::HearingNote;
use domain::identity::{Role, UserId};

#[test]
fn replay_returns_the_original_group_and_timestamp_without_document_admission() {
    let fixture = Fixture::single();
    let original = fixture.operation(at());
    let prepared = harness(
        fixture.replay_store(original.clone()),
        identity(fixture.actor.clone()),
    );
    assert_eq!(
        prepared
            .service
            .prepare("session", fixture.case_id, fixture.command.clone())
            .unwrap(),
        original.group.review
    );
    let submitted = harness(
        fixture.replay_store(original.clone()),
        identity(fixture.actor),
    );
    assert_eq!(
        submitted
            .service
            .submit(
                "session",
                fixture.case_id,
                fixture.command,
                confirmation(&original.group.review)
            )
            .unwrap(),
        original
    );
    for harness in [prepared, submitted] {
        assert_eq!(harness.validator.calls(), 0);
        assert!(harness.events().is_empty());
    }
}

#[test]
fn authorized_profile_changes_do_not_rewrite_the_historical_recording_principal() {
    let mut fixture = Fixture::single();
    let original = fixture.operation(at());
    fixture.actor.role = Role::Owner;
    fixture.actor.email = "current-owner@example.test".into();
    let harness = harness(
        fixture.replay_store(original.clone()),
        identity(fixture.actor.clone()),
    );
    let result = harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&original.group.review),
        )
        .unwrap();
    assert_eq!(result, original);
    assert_ne!(result.group.review.actor, fixture.actor);
}

#[test]
fn an_identical_raced_operation_returns_its_existing_capture_even_before_this_request() {
    let fixture = Fixture::single();
    let original = fixture.operation(at());
    let raced = original.clone();
    let mut store = fixture.store();
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
            confirmation(&original.group.review),
        )
        .unwrap();
    assert_eq!(result, original);
    assert!(result.group.recorded_at < now());
}

#[test]
fn both_confirmed_digests_are_required_for_fresh_submit_and_replay() {
    for replay in [false, true] {
        for review_digest in [false, true] {
            let fixture = Fixture::single();
            let original = fixture.operation(at());
            let mut expected = confirmation(&original.group.review);
            if review_digest {
                expected.review_digest = Sha256Digest::from_array([0x55; 32]);
            } else {
                expected.submission_digest = Sha256Digest::from_array([0x55; 32]);
            }
            let store = if replay {
                fixture.replay_store(original)
            } else {
                fixture.store()
            };
            let harness = harness(store, identity(fixture.actor));
            assert!(harness
                .service
                .submit("session", fixture.case_id, fixture.command, expected)
                .is_err());
        }
    }
}

#[test]
fn replay_rejects_changed_instruction_operation_decision_or_author_identity() {
    for field in 0..4 {
        let mut fixture = Fixture::single();
        let original = fixture.operation(at());
        match field {
            0 => {
                let mut input =
                    crate::measure_decision_fixtures::decision_input(&fixture.command.values);
                input.justification = HearingNote::new("Different decision declaration").unwrap();
                fixture.command.values = MeasureDecisionValues::new(input);
            }
            1 => {
                fixture.command.operation_id =
                    MeasureDecisionOperationId::from_uuid(Uuid::from_u128(900))
            }
            2 => fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(901)),
            _ => fixture.actor.id = UserId::from_uuid(Uuid::from_u128(902)),
        }
        let harness = harness(fixture.replay_store(original), identity(fixture.actor));
        assert!(harness
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
    }
}

#[test]
fn replay_reauthenticates_current_access_before_disclosure() {
    let fixture = Fixture::single();
    let original = fixture.operation(at());
    let mut identity = crate::case_support::MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    let actor = fixture.actor.clone();
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(actor));
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    let harness = harness(fixture.replay_store(original), identity);
    assert!(matches!(
        harness
            .service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::InvalidSession)
    ));
}
