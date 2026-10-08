use super::*;

fn list_returning(
    query: PrecautionaryHearingReadQuery,
    returned: PrecautionaryHearingRecordPage,
) -> Result<PrecautionaryHearingRecordPage, ApplicationError> {
    let actor = reader(Role::Paralegal);
    let expected_actor = actor.clone();
    let case = operation(10).capture.review.case_id;
    let mut store = MockReads::new();
    store.expect_list().times(1).return_once(move |a, c, q| {
        assert_eq!((a, c, q), (&expected_actor, case, query));
        Ok(returned)
    });
    service(store, identity(&actor), clock()).list("session", case, query)
}

#[test]
fn mixed_pages_have_exact_uuid_order_exclusive_cursor_and_continuation() {
    let first = operation(20);
    let second = operation(30);
    let case = first.capture.review.case_id;
    let query = PrecautionaryHearingReadQuery::new(2, Some(id(10))).unwrap();
    let mut page = page(case, vec![first, second]);
    page.has_more = true;
    page.next_after_id = Some(id(30));
    assert_eq!(list_returning(query, page.clone()).unwrap(), page);
    let empty = super::page(case, vec![]);
    assert_eq!(
        list_returning(
            PrecautionaryHearingReadQuery::new(2, Some(id(30))).unwrap(),
            empty.clone()
        )
        .unwrap(),
        empty
    );
}

#[test]
fn page_scope_order_duplicates_length_and_continuation_must_agree() {
    let first = operation(20);
    let second = operation(30);
    let query = PrecautionaryHearingReadQuery::new(2, Some(id(10))).unwrap();
    for change in 0..9 {
        let mut result = page(
            first.capture.review.case_id,
            vec![first.clone(), second.clone()],
        );
        match change {
            0 => result.case_id = CaseId::new(),
            1 => result.items.reverse(),
            2 => result.items[1] = first.clone(),
            3 => result.items.push(operation(40)),
            4 => result.items[0] = operation(10),
            5 => result.next_after_id = Some(id(30)),
            6 => {
                result.has_more = true;
                result.next_after_id = None;
            }
            7 => {
                result.has_more = true;
                result.next_after_id = Some(id(20));
            }
            _ => {
                result.items.pop();
                result.has_more = true;
                result.next_after_id = Some(id(20));
            }
        }
        assert!(
            list_returning(query, result).is_err(),
            "accepted page mutation {change}"
        );
    }
}

#[test]
fn exact_hearing_revision_operation_and_case_selectors_cannot_be_substituted() {
    let saved = operation(40);
    let actor = reader(Role::Owner);
    for kind in READS {
        let other = operation(50);
        let store = successful_store(&actor, &saved, other, kind);
        if matches!(kind, ReadKind::List) {
            assert!(read(&service(store, identity(&actor), clock()), &saved, kind).is_ok());
        } else {
            assert!(read(&service(store, identity(&actor), clock()), &saved, kind).is_err());
        }
        let mut foreign_request = saved.clone();
        foreign_request.capture.review.case_id = CaseId::new();
        let store = successful_store(&actor, &foreign_request, saved.clone(), kind);
        assert!(read(
            &service(store, identity(&actor), clock()),
            &foreign_request,
            kind
        )
        .is_err());
    }
    let later = cancelled(&saved);
    for kind in [ReadKind::Exact, ReadKind::Operation] {
        let store = successful_store(&actor, &saved, later.clone(), kind);
        assert!(read(&service(store, identity(&actor), clock()), &saved, kind).is_err());
    }
}
