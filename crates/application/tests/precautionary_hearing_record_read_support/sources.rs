use super::*;

fn rejects_pair(
    first: PrecautionaryHearingRecordStoredOperation,
    second: PrecautionaryHearingRecordStoredOperation,
) {
    for operation in [&first, &second] {
        precautionary_hearing_history_with_decision_history_matches(
            &Hasher,
            &operation.history.captures,
            &operation.history.origin,
            &operation.history.record_history,
        )
        .unwrap();
    }
    let case = first.capture.review.case_id;
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(page(case, vec![first, second])));
    assert!(service(store, identity(&reader(Role::Owner)), clock())
        .list("session", case, PrecautionaryHearingReadQuery::default())
        .is_err());
}

#[test]
fn individually_valid_hearings_cannot_reuse_one_operation_or_change_shared_support() {
    for change in 0..2 {
        let first = operation(10);
        let mut second = operation(20);
        if change == 0 {
            second.capture.review.command.operation_id = first.capture.review.command.operation_id;
        } else {
            second.capture.review.sources.support.name =
                "different-historical-appointment.pdf".into();
        }
        refresh(&mut second.capture);
        second.history.captures = vec![second.capture.clone()];
        second.history.origin = precautionary_hearing_origin_with_decision_history(
            &Hasher,
            &second.capture,
            &second.history.record_history,
        )
        .unwrap();
        rejects_pair(first, second);
    }
}

#[test]
fn different_g2_owners_cannot_claim_the_same_exact_measure_revision_across_a_page() {
    let first = operation(10);
    let mut judicial = judicial_fixture();
    judicial.identities(999);
    let group = judicial.capture();
    let second = scheduled(
        20,
        vec![reference_v2(&group.measures[0])],
        append_v2(&judicial.history, &group),
        group.recorded_at,
    );
    rejects_pair(first, second);
}

#[test]
fn repeated_g2_owner_copies_must_retain_the_same_full_recording_provenance() {
    let first = operation(10);
    let mut judicial = judicial_fixture();
    judicial.actor.email = "other-historical-actor@example.test".into();
    let group = judicial.capture();
    let second = scheduled(
        20,
        vec![reference_v2(&group.measures[0])],
        append_v2(&judicial.history, &group),
        group.recorded_at,
    );
    rejects_pair(first, second);
}
