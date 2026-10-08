use super::*;

#[test]
fn query_bounds_default_and_cursor_use_stable_measure_identity() {
    for limit in [0, 21, u16::MAX] {
        assert!(MeasureRecordReadQuery::new(limit, None).is_err());
    }
    for limit in [1, 20] {
        let query = MeasureRecordReadQuery::new(limit, Some(id(3001))).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.after_id(), Some(id(3001)));
    }
    assert_eq!(MeasureRecordReadQuery::default().limit(), 10);
    assert_eq!(MeasureRecordReadQuery::default().after_id(), None);
}

#[test]
fn pages_order_measure_heads_and_keep_exact_continuation_without_operation_ordering() {
    let first = initial(10);
    let mut fixture = root_fixture(20);
    fixture.command.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(1));
    let second = from_group(&fixture.capture(), &crate::effect_support::empty_history());
    let mut expected = page(first.case_id, vec![first, second]);
    expected.has_more = true;
    expected.next_after_id = Some(id(3020));
    let query = MeasureRecordReadQuery::new(2, Some(id(3001))).unwrap();
    assert_eq!(list_result(query, expected.clone()).unwrap(), expected);
    let empty = page(crate::participant_support::case_id(), vec![]);
    assert_eq!(list_result(query, empty.clone()).unwrap(), empty);
}

#[test]
fn page_scope_length_order_cursor_and_continuation_must_all_agree() {
    for variant in 0..11 {
        let mut returned = page(crate::participant_support::case_id(), vec![initial(10)]);
        let mut limit = 1;
        let mut cursor = None;
        match variant {
            0 => returned.case_id = crate::participant_support::other_case(),
            1 => returned.items.push(initial(20)),
            2 => {
                returned.has_more = true;
                returned.next_after_id = Some(id(3010));
                limit = 2;
            }
            3 => returned.has_more = true,
            4 => {
                returned.has_more = true;
                returned.next_after_id = Some(id(3020));
            }
            5 => returned.next_after_id = Some(id(3010)),
            6 => {
                returned.items.clear();
                returned.has_more = true;
            }
            7 => {
                returned.items.push(initial(10));
                limit = 2;
            }
            8 => {
                returned.items.insert(0, initial(20));
                limit = 2;
            }
            9 => cursor = Some(id(3010)),
            _ => cursor = Some(id(3020)),
        }
        assert!(list_result(
            MeasureRecordReadQuery::new(limit, cursor).unwrap(),
            returned
        )
        .is_err());
    }
}

#[test]
fn current_lookup_binds_id_and_exact_lookup_binds_id_revision_and_digest() {
    let original = initial(10);
    let actor = reader(Role::Paralegal);
    let other = initial(20);
    for kind in [ReadKind::Current, ReadKind::Exact] {
        let store = successful_store(&actor, &original, other.clone(), kind);
        assert!(read(&service(store, identity(&actor)), kind, &original).is_err());
    }
    for selected in [
        PrecautionaryMeasureRef::new(
            original.reference.id(),
            MeasureRevision::new(2).unwrap(),
            original.reference.digest(),
        ),
        PrecautionaryMeasureRef::new(
            original.reference.id(),
            original.reference.revision(),
            Sha256Digest::from_array([99; 32]),
        ),
    ] {
        let mut requested = original.clone();
        requested.reference = selected;
        let store = successful_store(&actor, &requested, original.clone(), ReadKind::Exact);
        assert!(read(
            &service(store, identity(&actor)),
            ReadKind::Exact,
            &requested
        )
        .is_err());
    }
}
