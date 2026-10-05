use super::*;
use domain::crypto::Sha256Digest;

fn refuse(fixture: Fixture) {
    let h = harness(fixture.store(), identity(fixture.actor));
    assert!(h
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}

#[test]
fn mixed_review_requires_exact_complete_owners_and_the_original_hearing_prefix() {
    let prior = Fixture::schedule().operation(at());
    for mutation in 0..6 {
        let mut fixture = Fixture::replace(&prior);
        match mutation {
            0 => fixture
                .material
                .record_history
                .records
                .judicial
                .groups
                .clear(),
            1 => fixture
                .material
                .record_history
                .records
                .administrative
                .clear(),
            2 => {
                fixture.material.record_history.records.administrative[0]
                    .origin
                    .capture_digest = Sha256Digest::from_array([9; 32])
            }
            3 => fixture.material.history.as_mut().unwrap().captures.clear(),
            4 => {
                fixture
                    .material
                    .history
                    .as_mut()
                    .unwrap()
                    .origin
                    .review_digest = Sha256Digest::from_array([9; 32])
            }
            _ => fixture
                .material
                .history
                .as_mut()
                .unwrap()
                .record_history
                .records
                .administrative
                .clear(),
        }
        refuse(fixture);
    }
}

#[test]
fn shared_owner_identity_cannot_merge_contradictory_source_material() {
    let prior = Fixture::schedule().operation(at());
    let mut fixture = Fixture::replace(&prior);
    fixture.material.record_history.records.administrative[0]
        .capture
        .review
        .actor
        .email = "contradictory@example.test".into();
    refuse(fixture);
}

#[test]
fn action_presence_and_exact_predecessor_are_validated_before_commit() {
    let prior = Fixture::schedule().operation(at());
    for mutation in 0..5 {
        let mut fixture = Fixture::replace(&prior);
        match mutation {
            0 => fixture.material.history = None,
            1 => fixture.material.selected_sources = None,
            2 => {
                fixture.command.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(999))
            }
            3 => {
                let PrecautionaryHearingChange::Replace {
                    expected_capture_digest,
                    ..
                } = &mut fixture.command.change
                else {
                    unreachable!()
                };
                *expected_capture_digest = Sha256Digest::from_array([9; 32]);
            }
            _ => {
                fixture.material.history.as_mut().unwrap().captures[0].capture_digest =
                    Sha256Digest::from_array([9; 32])
            }
        }
        refuse(fixture);
    }
    let mut fixture = Fixture::schedule();
    fixture.material.history = Some(prior.history.clone());
    refuse(fixture);
    let mut fixture = Fixture::cancel(&prior);
    fixture.material.selected_sources = Fixture::schedule().material.selected_sources;
    refuse(fixture);
}

#[test]
fn malformed_returned_capture_origin_or_mixed_closure_is_never_disclosed() {
    for mutation in 0..4 {
        let fixture = Fixture::schedule();
        let expected = confirmation(&fixture.review());
        let mut result = fixture.operation(now());
        match mutation {
            0 => result.capture.capture_digest = Sha256Digest::from_array([9; 32]),
            1 => result.history.origin.capture_digest = Sha256Digest::from_array([9; 32]),
            2 => result.history.record_history.records.administrative.clear(),
            _ => result.history.captures.clear(),
        }
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _| Ok(result));
        let h = harness(store, identity(fixture.actor));
        assert!(h
            .service
            .submit("session", fixture.case_id, fixture.command, expected)
            .is_err());
    }
}
