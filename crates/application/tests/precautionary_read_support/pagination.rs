use super::*;

#[test]
fn list_preserves_strict_uuid_order_and_exact_full_page_continuation() {
    let first = operation(20);
    let second = operation(30);
    let case_id = first.capture.review.case_id;
    let actor = first.capture.review.actor.clone();
    let mut expected = page(case_id, vec![first, second]);
    expected.has_more = true;
    expected.next_after_id = Some(id(30));
    let returned = expected.clone();
    let mut store = MockReads::new();
    let principal = actor.clone();
    let query = PrecautionaryHearingReadQuery::new(2, Some(id(10))).unwrap();
    store.expect_list().times(1).return_once(move |a, c, q| {
        assert_eq!((a, c, q), (&principal, case_id, query));
        Ok(returned)
    });
    assert_eq!(
        service(store, identity(&actor, 2), clock())
            .list("session", case_id, query)
            .unwrap(),
        expected,
    );
}

#[test]
fn empty_authorized_page_has_exact_case_scope_and_no_continuation() {
    let saved = operation(40);
    let case_id = saved.capture.review.case_id;
    let expected = page(case_id, vec![]);
    let returned = expected.clone();
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    assert_eq!(
        service(store, identity(&saved.capture.review.actor, 2), clock())
            .list("session", case_id, PrecautionaryHearingReadQuery::default())
            .unwrap(),
        expected,
    );
}

#[test]
fn malformed_page_order_scope_size_or_continuation_is_rejected() {
    let first = operation(20);
    let second = operation(30);
    let case_id = first.capture.review.case_id;
    let actor = first.capture.review.actor.clone();
    for mutation in 0..10 {
        let mut returned = page(case_id, vec![first.clone(), second.clone()]);
        match mutation {
            0 => returned.items.reverse(),
            1 => returned.items[1] = first.clone(),
            2 => returned.items.push(operation(40)),
            3 => returned.next_after_id = Some(id(30)),
            4 => returned.has_more = true,
            5 => {
                returned.has_more = true;
                returned.next_after_id = Some(id(20));
            }
            6 => {
                returned.items.pop();
                returned.has_more = true;
                returned.next_after_id = Some(id(20));
            }
            7 => {
                returned.items.clear();
                returned.has_more = true;
                returned.next_after_id = Some(id(30));
            }
            8 => {
                returned.items.clear();
                returned.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(999));
            }
            _ => {}
        }
        let mut store = MockReads::new();
        store
            .expect_list()
            .times(1)
            .return_once(move |_, _, _| Ok(returned));
        let after = if mutation == 9 { id(20) } else { id(10) };
        assert!(service(store, identity(&actor, 1), clock())
            .list(
                "session",
                case_id,
                PrecautionaryHearingReadQuery::new(2, Some(after)).unwrap(),
            )
            .is_err());
    }
}

#[test]
fn individually_valid_page_items_cannot_contradict_one_immutable_source() {
    let first = operation(20);
    let actor = first.capture.review.actor.clone();
    for mutation in 0..3 {
        let mut fixture = crate::receipt_support::Fixture::schedule();
        fixture.command.hearing_id = id(30);
        match mutation {
            0 => {
                crate::participant_support::manual_mut(&mut fixture.sources.participants[0])
                    .changed_by
                    .email = "contradictory recorder".into()
            }
            1 => fixture.sources.support.name = "other-captured-name.pdf".into(),
            _ => {
                fixture.sources.participants[1]
                    .bound_subject
                    .as_mut()
                    .unwrap()
                    .changed_by
                    .email = "contradictory subject recorder".into()
            }
        }
        let second = stored(fixture.capture(None, at()));
        precautionary_hearing_history_with_measure_history_matches(
            &Hasher,
            &second.history.captures,
            &second.history.origin,
            &second.history.measure_history,
        )
        .unwrap();
        let returned = page(first.capture.review.case_id, vec![first.clone(), second]);
        let mut store = MockReads::new();
        store
            .expect_list()
            .times(1)
            .return_once(move |_, _, _| Ok(returned));
        assert!(service(store, identity(&actor, 1), clock())
            .list(
                "session",
                first.capture.review.case_id,
                PrecautionaryHearingReadQuery::default(),
            )
            .is_err());
    }
}

#[test]
fn valid_page_scope_cannot_hide_an_individually_valid_foreign_case_item() {
    let saved = operation(40);
    let actor = saved.capture.review.actor.clone();
    let requested_case = CaseId::from_uuid(uuid::Uuid::from_u128(999));
    let returned = page(requested_case, vec![saved]);
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    assert!(service(store, identity(&actor, 1), clock())
        .list(
            "session",
            requested_case,
            PrecautionaryHearingReadQuery::default(),
        )
        .is_err());
}
