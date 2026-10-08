use super::*;

#[test]
fn fresh_and_replayed_operations_require_both_confirmation_digests() {
    for replay in [false, true] {
        for review in [false, true] {
            let fixture = Fixture::single();
            let mut expected = confirmation(&fixture.review());
            if review {
                expected.review_digest = Sha256Digest::from_array([99; 32]);
            } else {
                expected.submission_digest = Sha256Digest::from_array([99; 32]);
            }
            let store = if replay {
                fixture.replay_store(fixture.operation(at()))
            } else {
                fixture.store()
            };
            let harness = harness(store, identity(fixture.actor));
            let result =
                harness
                    .service
                    .submit("session", fixture.case_id, fixture.command, expected);
            if review {
                assert!(matches!(
                    result,
                    Err(ApplicationError::MeasureAdministrative(
                        MeasureAdministrativeError::ReviewMismatch
                    ))
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(ApplicationError::MeasureAdministrative(
                        MeasureAdministrativeError::SubmissionMismatch
                    ))
                ));
            }
        }
    }
}

#[test]
fn ready_case_and_exact_context_expectation_cannot_be_substituted() {
    for mutation in 0..2 {
        let mut fixture = Fixture::single();
        if mutation == 0 {
            fixture.case_id = CaseId::from_uuid(Uuid::from_u128(999));
        } else {
            fixture.command.context.context_digest = Sha256Digest::from_array([99; 32]);
        }
        reject_before_admission(fixture);
    }
}

#[test]
fn a_valid_committed_administrative_instruction_cannot_replace_the_confirmed_one() {
    let fixture = Fixture::single();
    let mut different = fixture.clone();
    different.command.reason = note("A different administrative instruction");
    let returned = different.operation(now());
    measure_administrative_capture_with_decision_history_matches(
        &Hasher,
        &returned.capture,
        &returned.record_history,
    )
    .unwrap();
    for replay in [false, true] {
        reject_returned(fixture.clone(), returned.clone(), replay);
    }
}
