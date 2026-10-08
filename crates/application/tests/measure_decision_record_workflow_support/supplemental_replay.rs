use super::*;

fn confirmation_v1(review: &MeasureDecisionReview) -> MeasureDecisionConfirmation {
    MeasureDecisionConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    }
}

#[test]
fn both_replay_families_retain_original_profile_after_current_authorized_profile_changes() {
    let fixture = Fixture::corrected();
    for original in [
        MeasureDecisionRecordReceipt::V1(Box::new(fixture.legacy_operation())),
        MeasureDecisionRecordReceipt::V2(Box::new(fixture.operation(now() - Duration::seconds(1)))),
    ] {
        let (command, expected) = match &original {
            MeasureDecisionRecordReceipt::V1(value) => (
                value.group.review.command.clone(),
                confirmation_v1(&value.group.review),
            ),
            MeasureDecisionRecordReceipt::V2(value) => (
                value.group.review.command.clone(),
                confirmation(&value.group.review),
            ),
        };
        let mut actor = fixture.actor.clone();
        actor.role = Role::Owner;
        actor.email = "current-owner@example.test".into();
        let harness = harness(replay_store(original.clone()), identity(actor));
        let actual = harness
            .service
            .submit("session", fixture.case_id, command, expected)
            .unwrap();
        assert_eq!(actual, original);
        assert_eq!(harness.validator.calls(), 0);
        assert!(harness.events().is_empty());
    }
}

#[test]
fn both_confirmed_digests_are_required_for_fresh_and_each_original_family() {
    for replay in 0..3 {
        for review_digest in [false, true] {
            let fixture = Fixture::corrected();
            let (store, command, mut expected) = match replay {
                0 => (
                    fixture.store(),
                    fixture.command.clone(),
                    confirmation(&fixture.review()),
                ),
                1 => {
                    let original = fixture.legacy_operation();
                    let command = original.group.review.command.clone();
                    let expected = confirmation_v1(&original.group.review);
                    (
                        replay_store(MeasureDecisionRecordReceipt::V1(Box::new(original))),
                        command,
                        expected,
                    )
                }
                _ => {
                    let original = fixture.operation(now() - Duration::seconds(1));
                    (
                        replay_store(MeasureDecisionRecordReceipt::V2(Box::new(original))),
                        fixture.command.clone(),
                        confirmation(&fixture.review()),
                    )
                }
            };
            if review_digest {
                expected.review_digest = Sha256Digest::from_array([77; 32]);
            } else {
                expected.submission_digest = Sha256Digest::from_array([77; 32]);
            }
            let harness = harness(store, identity(fixture.actor));
            assert!(matches!(
                harness
                    .service
                    .submit("session", fixture.case_id, command, expected),
                Err(ApplicationError::MeasureDecision(
                    MeasureDecisionError::ReviewMismatch
                )) | Err(ApplicationError::MeasureDecision(
                    MeasureDecisionError::SubmissionMismatch
                ))
            ));
        }
    }
}

#[test]
fn original_v1_confirmation_cannot_confirm_fresh_v2_with_identical_instruction() {
    let fixture = initial();
    let review = fixture.review();
    let legacy = prepare_measure_decision_capture(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command.clone(),
        MeasureDecisionMaterial {
            context: review.material.context.clone(),
            support: review.material.support.clone(),
            anchor: None,
            predecessors: vec![],
            result_sources: review.material.result_sources.clone(),
        },
    )
    .unwrap()
    .into_group_capture(&Hasher, now() - Duration::seconds(1))
    .unwrap();
    assert_eq!(legacy.review.submission_digest, review.submission_digest);
    assert_ne!(legacy.review.review_digest, review.review_digest);
    let harness = harness(fixture.store(), identity(fixture.actor));
    assert!(matches!(
        harness.service.submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation_v1(&legacy.review)
        ),
        Err(ApplicationError::MeasureDecision(
            MeasureDecisionError::ReviewMismatch
        ))
    ));
}

#[test]
fn exact_raced_v2_replay_preserves_old_capture_but_raced_v1_conflict_is_not_retried() {
    for conflict in [false, true] {
        let fixture = Fixture::corrected();
        let original = fixture.operation(now() - Duration::seconds(1));
        let expected = confirmation(&original.group.review);
        let returned = original.clone();
        let mut store = fixture.store();
        store.expect_commit().times(1).return_once(move |_, _, _| {
            if conflict {
                Err(MeasureDecisionError::OperationConflict.into())
            } else {
                Ok(returned)
            }
        });
        let harness = harness(store, identity(fixture.actor));
        let result = harness
            .service
            .submit("session", fixture.case_id, fixture.command, expected);
        if conflict {
            assert!(matches!(
                result,
                Err(ApplicationError::MeasureDecision(
                    MeasureDecisionError::OperationConflict
                ))
            ));
        } else {
            assert_eq!(
                result.unwrap(),
                MeasureDecisionRecordReceipt::V2(Box::new(original))
            );
        }
    }
}

#[test]
fn replay_rejects_different_instruction_identity_actor_and_malformed_origin() {
    for field in 0..5 {
        let mut fixture = Fixture::corrected();
        let mut original = fixture.operation(now() - Duration::seconds(1));
        match field {
            0 => {
                let mut input =
                    crate::measure_decision_fixtures::decision_input(&fixture.command.values);
                input.justification = crate::correction_support::note("Another declaration");
                fixture.command.values = MeasureDecisionValues::new(input);
            }
            1 => fixture.command.operation_id = MeasureDecisionOperationId::new(),
            2 => fixture.command.decision_id = MeasureDecisionId::new(),
            3 => fixture.actor.id = UserId::new(),
            _ => original.origin.review_digest = Sha256Digest::from_array([66; 32]),
        }
        let harness = harness(
            replay_store(MeasureDecisionRecordReceipt::V2(Box::new(original))),
            identity(fixture.actor),
        );
        assert!(harness
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert!(harness.events().is_empty());
    }
}
