use super::*;
use domain::crypto::Sha256Digest;
use domain::identity::{Role, UserId};

#[test]
fn exact_replay_preserves_original_actor_profile_and_skips_admission() {
    let mut fixture = Fixture::schedule();
    let original = fixture.operation(at());
    fixture.actor.email = "current-owner@example.test".into();
    fixture.actor.role = Role::Owner;
    for submit in [false, true] {
        let h = harness(
            fixture.replay_store(original.clone()),
            identity(fixture.actor.clone()),
        );
        if submit {
            assert_eq!(
                h.service
                    .submit(
                        "session",
                        fixture.case_id,
                        fixture.command.clone(),
                        confirmation(&original.capture.review)
                    )
                    .unwrap(),
                original
            );
        } else {
            assert_eq!(
                h.service
                    .prepare("session", fixture.case_id, fixture.command.clone())
                    .unwrap(),
                original.capture.review
            );
        }
        assert_eq!(h.validator.calls(), 0);
        assert!(h.events().is_empty());
    }
}

#[test]
fn both_confirmation_digests_are_required_for_fresh_and_replayed_operations() {
    for replay in [false, true] {
        for review_digest in [false, true] {
            let fixture = Fixture::schedule();
            let original = fixture.operation(at());
            let mut expected = confirmation(&original.capture.review);
            if review_digest {
                expected.review_digest = Sha256Digest::from_array([55; 32]);
            } else {
                expected.submission_digest = Sha256Digest::from_array([55; 32]);
            }
            let store = if replay {
                fixture.replay_store(original)
            } else {
                fixture.store()
            };
            let h = harness(store, identity(fixture.actor));
            assert!(h
                .service
                .submit("session", fixture.case_id, fixture.command, expected)
                .is_err());
        }
    }
}

#[test]
fn replay_requires_exact_instruction_operation_case_and_recording_user() {
    for mutation in 0..4 {
        let mut fixture = Fixture::schedule();
        let original = fixture.operation(at());
        match mutation {
            0 => fixture.actor.id = UserId::from_uuid(Uuid::from_u128(777)),
            1 => fixture.case_id = CaseId::from_uuid(Uuid::from_u128(778)),
            2 => {
                fixture.command.operation_id =
                    PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(779))
            }
            _ => {
                let PrecautionaryHearingChange::Schedule { values, .. } =
                    &mut fixture.command.change
                else {
                    unreachable!()
                };
                let mut input = values_input(values);
                input.venue = domain::hearings::HearingVenue::new("A different venue").unwrap();
                *values = PrecautionaryHearingValues::new(input).unwrap();
            }
        }
        let h = harness(fixture.replay_store(original), identity(fixture.actor));
        assert!(h
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert!(h.events().is_empty());
    }
}

#[test]
fn exact_raced_replay_keeps_its_original_timestamp_earlier_than_the_precommit_floor() {
    let fixture = Fixture::schedule();
    let original = fixture.operation(at());
    let confirmation = confirmation(&original.capture.review);
    let raced = original.clone();
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, prepared| {
            assert_eq!(prepared.review(), &raced.capture.review);
            Ok(raced)
        });
    let h = harness(store, identity(fixture.actor));
    assert_eq!(
        h.service
            .submit("session", fixture.case_id, fixture.command, confirmation)
            .unwrap(),
        original
    );
}
