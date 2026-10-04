use super::*;

#[test]
fn exact_get_rejects_another_case_hearing_or_revision() {
    let saved = operation(40);
    let actor = &saved.capture.review.actor;
    for mutation in 0..3 {
        let mut request = saved.clone();
        match mutation {
            0 => request.capture.review.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(999)),
            1 => request.capture.review.command.hearing_id = id(999),
            _ => {
                request.capture.review.result_revision =
                    PrecautionaryHearingRevision::new(2).unwrap()
            }
        }
        let store = successful_store(actor, &request, saved.clone(), ReadKind::Get);
        assert!(read(
            &service(store, identity(actor, 1), clock()),
            &request,
            ReadKind::Get,
        )
        .is_err());
    }
}

#[test]
fn operation_lookup_requires_exact_case_and_operation_identity() {
    let saved = operation(40);
    let actor = &saved.capture.review.actor;
    for mutation in 0..2 {
        let mut request = saved.clone();
        if mutation == 0 {
            request.capture.review.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(999));
        } else {
            request.capture.review.command.operation_id =
                PrecautionaryHearingOperationId::from_uuid(uuid::Uuid::from_u128(999));
        }
        let store = successful_store(actor, &request, saved.clone(), ReadKind::Operation);
        assert!(read(
            &service(store, identity(actor, 1), clock()),
            &request,
            ReadKind::Operation,
        )
        .is_err());
    }
}

#[test]
fn get_without_revision_preserves_the_store_head_selection_and_complete_history() {
    let initial = operation(40);
    let expected = replaced(&initial);
    let returned = expected.clone();
    let mut store = MockReads::new();
    store
        .expect_get()
        .times(1)
        .return_once(move |_, _, _, revision| {
            assert_eq!(revision, None);
            Ok(returned)
        });
    assert_eq!(
        service(store, identity(&initial.capture.review.actor, 2), clock())
            .get(
                "session",
                initial.capture.review.case_id,
                initial.capture.review.command.hearing_id,
                None,
            )
            .unwrap(),
        expected,
    );
}

#[test]
fn every_read_validates_the_complete_prefix_origin_and_separate_operation_capture() {
    let saved = replaced(&operation(40));
    let actor = &saved.capture.review.actor;
    for kind in READS {
        for mutation in 0..7 {
            let mut returned = saved.clone();
            match mutation {
                0 => returned.history.captures.clear(),
                1 => returned.history.origin.capture_digest = Sha256Digest::from_array([99; 32]),
                2 => {
                    returned.history.captures.remove(0);
                }
                3 => returned.history.captures.reverse(),
                4 => {
                    returned.history.captures[0].capture_digest = Sha256Digest::from_array([99; 32])
                }
                5 => returned.capture = returned.history.captures[0].clone(),
                _ => {
                    returned.capture.review.sources.support.digest =
                        Sha256Digest::from_array([99; 32])
                }
            }
            let store = successful_store(actor, &saved, returned, kind);
            assert!(read(&service(store, identity(actor, 1), clock()), &saved, kind).is_err());
        }
    }
}

#[test]
fn rehashed_copies_of_one_source_revision_cannot_change_inside_read_history() {
    let saved = replaced(&operation(40));
    let actor = &saved.capture.review.actor;
    for kind in READS {
        let mut returned = saved.clone();
        crate::participant_support::manual_mut(
            &mut returned.capture.review.sources.participants[0],
        )
        .changed_by
        .email = "different historical recorder".into();
        refresh(&mut returned.capture);
        *returned.history.captures.last_mut().unwrap() = returned.capture.clone();
        precautionary_hearing_receipt_matches(&Hasher, &returned.capture).unwrap();
        let store = successful_store(actor, &saved, returned, kind);
        assert!(read(&service(store, identity(actor, 1), clock()), &saved, kind).is_err());
    }
}

#[test]
fn rehashed_review_target_claim_requires_actual_measure_history_at_every_read() {
    let mut returned = operation(40);
    let mut input = crate::receipt_support::values_input();
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = vec![PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(uuid::Uuid::from_u128(999)),
        MeasureRevision::initial(),
        Sha256Digest::from_array([99; 32]),
    )];
    let values = PrecautionaryHearingValues::new(input).unwrap();
    returned.capture.review.resolved_values = values.clone();
    let PrecautionaryHearingChange::Schedule {
        values: selected, ..
    } = &mut returned.capture.review.command.change
    else {
        unreachable!()
    };
    *selected = values;
    refresh(&mut returned.capture);
    returned.history.captures[0] = returned.capture.clone();
    returned.history.origin.submission_digest = returned.capture.review.submission_digest;
    returned.history.origin.review_digest = returned.capture.review.review_digest;
    returned.history.origin.capture_digest = returned.capture.capture_digest;
    let actor = &returned.capture.review.actor;
    for kind in READS {
        let store = successful_store(actor, &returned, returned.clone(), kind);
        assert!(read(
            &service(store, identity(actor, 1), clock()),
            &returned,
            kind
        )
        .is_err());
    }
}

#[test]
fn authorized_read_preserves_a_genuine_historical_cancellation() {
    let initial = operation(40);
    let fixture = crate::receipt_support::Fixture::cancel(&initial.capture);
    let capture = fixture.capture(Some(&initial.capture), at() + time::Duration::seconds(2));
    let mut saved = initial;
    saved.history.captures.push(capture.clone());
    saved.capture = capture;
    precautionary_hearing_history_with_measure_history_matches(
        &Hasher,
        &saved.history.captures,
        &saved.history.origin,
        &saved.history.measure_history,
    )
    .unwrap();
    for kind in READS {
        let actor = &saved.capture.review.actor;
        let store = successful_store(actor, &saved, saved.clone(), kind);
        assert_eq!(
            read(&service(store, identity(actor, 2), clock()), &saved, kind).unwrap(),
            vec![saved.clone()],
        );
    }
}

#[test]
fn full_hearing_prefix_budget_is_enforced_before_read_disclosure() {
    let saved = operation(40);
    let mut returned = saved.clone();
    returned.history.captures = vec![saved.capture.clone(); 257];
    for kind in READS {
        let actor = &saved.capture.review.actor;
        let store = successful_store(actor, &saved, returned.clone(), kind);
        assert!(matches!(
            read(&service(store, identity(actor, 1), clock()), &saved, kind),
            Err(ApplicationError::PrecautionaryHearing(
                PrecautionaryHearingError::IncompleteHistory
            ))
        ));
    }
}
