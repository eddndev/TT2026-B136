use super::*;
use domain::crypto::Sha256Digest;
use domain::hearings::HearingVenue;
use domain::identity::UserId;
use time::Duration;
use uuid::Uuid;

#[test]
fn submission_and_review_digest_mismatches_each_prevent_commit() {
    for review_digest in [false, true] {
        let fixture = Fixture::schedule();
        let mut expected = confirmation(&fixture.review());
        if review_digest {
            expected.review_digest = Sha256Digest::from_array([0x55; 32]);
        } else {
            expected.submission_digest = Sha256Digest::from_array([0x55; 32]);
        }
        let harness = harness(fixture.store(), identity(fixture.actor));
        assert!(harness
            .service
            .submit("session", fixture.case_id, fixture.command, expected)
            .is_err());
    }
}

#[test]
fn replay_requires_both_original_confirmation_digests() {
    for review_digest in [false, true] {
        let fixture = Fixture::schedule();
        let original = fixture.operation(at());
        let mut expected = confirmation(&original.capture.review);
        if review_digest {
            expected.review_digest = Sha256Digest::from_array([0x55; 32]);
        } else {
            expected.submission_digest = Sha256Digest::from_array([0x55; 32]);
        }
        let harness = harness(fixture.replay_store(original), identity(fixture.actor));
        assert!(harness
            .service
            .submit("session", fixture.case_id, fixture.command, expected)
            .is_err());
    }
}

#[test]
fn matching_operation_uuid_does_not_allow_a_changed_instruction_to_replay() {
    let mut fixture = Fixture::schedule();
    let original = fixture.operation(at());
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut fixture.command.change else {
        unreachable!()
    };
    let mut input = values_input(values);
    input.venue = HearingVenue::new("Different communicated venue").unwrap();
    *values = PrecautionaryHearingValues::new(input).unwrap();
    let harness = harness(fixture.replay_store(original), identity(fixture.actor));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}

#[test]
fn replay_cannot_disclose_another_recording_users_operation() {
    let mut fixture = Fixture::schedule();
    let original = fixture.operation(at());
    fixture.actor.id = UserId::from_uuid(Uuid::from_u128(777));
    let harness = harness(fixture.replay_store(original), identity(fixture.actor));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}

#[test]
fn same_values_with_a_different_operation_id_do_not_qualify_as_replay() {
    let mut fixture = Fixture::schedule();
    let original = fixture.operation(at());
    fixture.command.operation_id = PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(778));
    let harness = harness(fixture.replay_store(original), identity(fixture.actor));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}

#[test]
fn replace_and_cancel_prepare_and_replay_use_the_exact_original_history() {
    let initial = Fixture::schedule().operation(at());
    let replaced = Fixture::replace(&initial).operation(at() + Duration::seconds(2));
    for fixture in [Fixture::replace(&initial), Fixture::cancel(&replaced)] {
        let review = fixture.review();
        let harness = harness(fixture.store(), identity(fixture.actor.clone()));
        assert_eq!(
            harness
                .service
                .prepare("session", fixture.case_id, fixture.command.clone())
                .unwrap(),
            review
        );
        let original = fixture.operation(at() + Duration::seconds(4));
        let harness = super::harness(
            fixture.replay_store(original.clone()),
            identity(fixture.actor),
        );
        assert_eq!(
            harness
                .service
                .submit(
                    "session",
                    fixture.case_id,
                    fixture.command,
                    confirmation(&review),
                )
                .unwrap(),
            original
        );
    }
}

#[test]
fn replay_reauthenticates_the_current_principal_before_disclosure() {
    let fixture = Fixture::schedule();
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
        Err(ApplicationError::InvalidSession),
    ));
}
