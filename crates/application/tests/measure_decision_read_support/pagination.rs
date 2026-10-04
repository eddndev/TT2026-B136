use super::*;

fn list_result(
    query: MeasureDecisionReadQuery,
    returned: MeasureDecisionPage,
) -> Result<MeasureDecisionPage, ApplicationError> {
    let actor = reader(Role::Paralegal);
    let case_id = crate::participant_support::case_id();
    let mut store = MockReads::new();
    let expected_actor = actor.clone();
    store
        .expect_list()
        .times(1)
        .return_once(move |a, c, actual_query| {
            assert_eq!((a, c), (&expected_actor, case_id));
            assert_eq!(actual_query, query);
            Ok(returned)
        });
    service(store, identity(&actor)).list("session", case_id, query)
}

#[test]
fn query_has_a_bounded_limit_and_preserves_its_exact_decision_cursor() {
    for limit in [0, 21, u16::MAX] {
        assert!(MeasureDecisionReadQuery::new(limit, None).is_err());
    }
    for limit in [1, 20] {
        let query = MeasureDecisionReadQuery::new(limit, Some(id(10))).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.after_id(), Some(id(10)));
    }
    assert_eq!(MeasureDecisionReadQuery::default().limit(), 10);
    assert_eq!(MeasureDecisionReadQuery::default().after_id(), None);
}

#[test]
fn an_ascending_full_page_uses_the_last_returned_decision_as_continuation() {
    let rows = vec![operation(20), operation(30)];
    let case_id = rows[0].group.review.case_id;
    let mut expected = page(case_id, rows);
    expected.has_more = true;
    expected.next_after_id = Some(id(30));
    let query = MeasureDecisionReadQuery::new(2, Some(id(10))).unwrap();
    assert_eq!(list_result(query, expected.clone()).unwrap(), expected);
}

#[test]
fn an_empty_final_page_has_no_continuation() {
    let expected = page(crate::participant_support::case_id(), vec![]);
    let query = MeasureDecisionReadQuery::new(2, Some(id(10))).unwrap();
    assert_eq!(list_result(query, expected.clone()).unwrap(), expected);
}

#[test]
fn page_length_cannot_exceed_the_requested_limit() {
    let returned = page(
        crate::participant_support::case_id(),
        vec![operation(10), operation(20)],
    );
    assert!(list_result(MeasureDecisionReadQuery::new(1, None).unwrap(), returned).is_err());
}

#[test]
fn impossible_continuation_metadata_is_rejected() {
    for variant in 0..4 {
        let mut returned = page(crate::participant_support::case_id(), vec![operation(10)]);
        let limit = match variant {
            0 => {
                returned.has_more = true;
                returned.next_after_id = Some(id(10));
                2
            }
            1 => {
                returned.has_more = true;
                1
            }
            2 => {
                returned.has_more = true;
                returned.next_after_id = Some(id(20));
                1
            }
            _ => {
                returned.next_after_id = Some(id(10));
                1
            }
        };
        assert!(list_result(
            MeasureDecisionReadQuery::new(limit, None).unwrap(),
            returned
        )
        .is_err());
    }
}

#[test]
fn duplicate_reversed_or_not_after_cursor_rows_are_rejected() {
    for (rows, cursor) in [
        (vec![operation(10), operation(10)], None),
        (vec![operation(20), operation(10)], None),
        (vec![operation(10)], Some(id(10))),
        (vec![operation(10)], Some(id(20))),
    ] {
        let returned = page(crate::participant_support::case_id(), rows);
        assert!(list_result(MeasureDecisionReadQuery::new(2, cursor).unwrap(), returned).is_err());
    }
}

#[test]
fn a_page_cannot_claim_another_case_even_when_its_rows_are_valid() {
    let returned = page(
        crate::participant_support::other_case(),
        vec![operation(10)],
    );
    assert!(list_result(MeasureDecisionReadQuery::default(), returned).is_err());
}
