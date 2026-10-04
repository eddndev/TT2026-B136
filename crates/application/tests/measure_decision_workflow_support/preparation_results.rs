use super::*;

fn reject_returned(fixture: Fixture, returned: MeasureDecisionStoredOperation, replay: bool) {
    let expected = confirmation(&fixture.review());
    let store = if replay {
        fixture.replay_store(returned)
    } else {
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _| Ok(returned));
        store
    };
    let harness = harness(store, identity(fixture.actor));
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}

fn refresh_claims(operation: &mut MeasureDecisionStoredOperation) {
    crate::measure_decision_fixtures::refresh_digests(&mut operation.group);
    operation.origin.submission_digest = operation.group.review.submission_digest;
    operation.origin.review_digest = operation.group.review.review_digest;
    operation.origin.decision_digest = operation.group.decision.capture_digest;
    operation.origin.group_digest = operation.group.capture_digest;
}

#[test]
fn replay_and_commit_verify_every_field_of_the_returned_group_origin() {
    for replay in [false, true] {
        for mutation in 0..7 {
            let fixture = Fixture::single();
            let mut returned = fixture.operation(now());
            match mutation {
                0 => returned.origin.case_id = CaseId::from_uuid(Uuid::from_u128(999)),
                1 => {
                    returned.origin.operation_id =
                        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999))
                }
                2 => {
                    returned.origin.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(999))
                }
                3 => returned.origin.submission_digest = Sha256Digest::from_array([99; 32]),
                4 => returned.origin.review_digest = Sha256Digest::from_array([99; 32]),
                5 => returned.origin.decision_digest = Sha256Digest::from_array([99; 32]),
                _ => returned.origin.group_digest = Sha256Digest::from_array([99; 32]),
            }
            reject_returned(fixture, returned, replay);
        }
    }
}

#[test]
fn returned_rehashed_group_cannot_omit_or_append_real_measure_members() {
    for replay in [false, true] {
        let fixture = Fixture::multiple();
        let mut missing = fixture.operation(now());
        missing.group.measures.pop();
        refresh_claims(&mut missing);
        reject_returned(fixture, missing, replay);

        let fixture = Fixture::single();
        let mut extra = fixture.operation(now());
        let genuine_other_member = Fixture::multiple()
            .operation(now())
            .group
            .measures
            .remove(1);
        extra.group.measures.push(genuine_other_member);
        refresh_claims(&mut extra);
        reject_returned(fixture, extra, replay);
    }
}

#[test]
fn returned_operation_requires_its_complete_exact_ancestor_closure() {
    let previous = Fixture::single().operation(at());
    for replay in [false, true] {
        for mutation in 0..4 {
            let fixture = Fixture::confirm(&previous);
            let mut returned = fixture.operation(now());
            let groups = &mut returned.measure_history.groups;
            match mutation {
                0 => groups.clear(),
                1 => groups[0].origin.group_digest = Sha256Digest::from_array([99; 32]),
                2 => groups[0].capture.measures.clear(),
                _ => groups.push(groups[0].clone()),
            }
            reject_returned(fixture, returned, replay);
        }
    }
}

#[test]
fn a_valid_returned_group_with_an_unrequested_measure_is_rejected() {
    let returned = Fixture::multiple().operation(now());
    measure_decision_group_with_history_matches(
        &Hasher,
        &returned.group,
        &returned.measure_history,
    )
    .unwrap();
    assert_eq!(returned.group.measures.len(), 2);
    for replay in [false, true] {
        reject_returned(Fixture::single(), returned.clone(), replay);
    }
}

#[test]
fn a_valid_commit_cannot_substitute_captured_sources_under_the_confirmed_command() {
    let fixture = Fixture::single();
    let confirmed = fixture.review();
    let mut substituted = fixture.clone();
    substituted.material.result_sources[0]
        .sources
        .subject
        .changed_by
        .email = "another captured subject recorder".into();
    let returned = substituted.operation(now());
    assert_eq!(returned.group.review.command, confirmed.command);
    assert_eq!(
        returned.group.review.submission_digest,
        confirmed.submission_digest
    );
    assert_ne!(returned.group.review.review_digest, confirmed.review_digest);
    measure_decision_group_with_history_matches(
        &Hasher,
        &returned.group,
        &returned.measure_history,
    )
    .unwrap();
    reject_returned(fixture, returned, false);
}

#[test]
fn returned_substitution_must_preserve_all_atomic_group_links() {
    let previous = Fixture::single().operation(at());
    for replay in [false, true] {
        let fixture = Fixture::substitute(&previous);
        let mut returned = fixture.operation(now());
        returned.group.substitutions[0].successors.pop();
        refresh_claims(&mut returned);
        reject_returned(fixture, returned, replay);
    }
}
