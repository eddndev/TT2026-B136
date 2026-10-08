use super::*;

#[test]
fn query_bounds_and_default_preserve_the_exact_operation_cursor() {
    for limit in [0, 21, u16::MAX] {
        assert!(MeasureAdministrativeReadQuery::new(limit, None).is_err());
    }
    for limit in [1, 20] {
        let query = MeasureAdministrativeReadQuery::new(limit, Some(operation_id(10))).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.after_operation_id(), Some(operation_id(10)));
    }
    assert_eq!(MeasureAdministrativeReadQuery::default().limit(), 10);
    assert_eq!(
        MeasureAdministrativeReadQuery::default().after_operation_id(),
        None
    );
}

#[test]
fn ascending_full_page_has_exact_last_operation_continuation_and_empty_final_page_is_valid() {
    let case_id = crate::participant_support::case_id();
    let mut expected = page(case_id, vec![operation(20), operation(30)]);
    expected.has_more = true;
    expected.next_after_operation_id = Some(operation_id(30));
    let query = MeasureAdministrativeReadQuery::new(2, Some(operation_id(10))).unwrap();
    assert_eq!(list_result(query, expected.clone()).unwrap(), expected);
    let empty = page(case_id, vec![]);
    assert_eq!(list_result(query, empty.clone()).unwrap(), empty);
}

#[test]
fn page_scope_length_and_continuation_are_checked_before_disclosure() {
    let case_id = crate::participant_support::case_id();
    for variant in 0..7 {
        let mut returned = page(case_id, vec![operation(10)]);
        let limit = match variant {
            0 => {
                returned.case_id = crate::participant_support::other_case();
                1
            }
            1 => {
                returned.items.push(operation(20));
                1
            }
            2 => {
                returned.has_more = true;
                returned.next_after_operation_id = Some(operation_id(10));
                2
            }
            3 => {
                returned.has_more = true;
                1
            }
            4 => {
                returned.has_more = true;
                returned.next_after_operation_id = Some(operation_id(20));
                1
            }
            5 => {
                returned.next_after_operation_id = Some(operation_id(10));
                1
            }
            _ => {
                returned.items.clear();
                returned.has_more = true;
                1
            }
        };
        assert!(list_result(
            MeasureAdministrativeReadQuery::new(limit, None).unwrap(),
            returned
        )
        .is_err());
    }
}

#[test]
fn duplicate_reversed_and_nonexclusive_operation_rows_are_rejected() {
    let case_id = crate::participant_support::case_id();
    for (rows, cursor) in [
        (vec![operation(10), operation(10)], None),
        (vec![operation(20), operation(10)], None),
        (vec![operation(10)], Some(operation_id(10))),
        (vec![operation(10)], Some(operation_id(20))),
    ] {
        let query = MeasureAdministrativeReadQuery::new(2, cursor).unwrap();
        assert!(list_result(query, page(case_id, rows)).is_err());
    }
}

#[test]
fn operation_lookup_cannot_return_another_valid_original_receipt() {
    let expected = operation(10);
    let returned = operation(20);
    let actor = reader(Role::Paralegal);
    let store = successful_store(&actor, &expected, returned, ReadKind::Operation);
    assert!(read(
        &service(store, identity(&actor)),
        ReadKind::Operation,
        &expected
    )
    .is_err());
}
