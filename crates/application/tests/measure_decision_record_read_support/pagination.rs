use super::*;

#[test]
fn mixed_pages_preserve_actual_families_shared_ancestry_and_uuid_continuation() {
    let first = v1(10);
    let second = v2(10);
    let case = first.case_id();
    let query = MeasureDecisionReadQuery::new(2, None).unwrap();
    let mut expected = page(case, vec![first, second]);
    expected.has_more = true;
    expected.next_after_id = Some(expected.items[1].origin().decision_id);
    assert_eq!(list_result(query, expected.clone()).unwrap(), expected);
    let cursor = expected.next_after_id;
    let empty = page(case, vec![]);
    assert_eq!(
        list_result(
            MeasureDecisionReadQuery::new(2, cursor).unwrap(),
            empty.clone()
        )
        .unwrap(),
        empty
    );
}

#[test]
fn mixed_pages_reject_wrong_scope_order_length_cursor_or_continuation() {
    let first = v1(10);
    let second = v2(10);
    let case = first.case_id();
    for mutation in 0..8 {
        let mut returned = page(case, vec![first.clone(), second.clone()]);
        let mut query = MeasureDecisionReadQuery::new(2, None).unwrap();
        match mutation {
            0 => returned.case_id = CaseId::from_uuid(Uuid::from_u128(999)),
            1 => returned.items.reverse(),
            2 => returned.items[1] = first.clone(),
            3 => returned.items.push(second.clone()),
            4 => {
                query = MeasureDecisionReadQuery::new(2, Some(first.origin().decision_id)).unwrap()
            }
            5 => returned.next_after_id = Some(second.origin().decision_id),
            6 => returned.has_more = true,
            _ => {
                returned.items.pop();
                returned.has_more = true;
                returned.next_after_id = Some(first.origin().decision_id);
            }
        }
        assert!(list_result(query, returned).is_err());
    }
}

#[test]
fn zero_measure_change_v2_decisions_remain_readable_list_items() {
    let mut fixture = FixtureV2::initial(root_fixture(20));
    fixture.command.outcome = MeasureDecisionOutcome::new(
        MeasureDecisionOutcomeInput::NoMeasureChange(note("No change declared")),
    )
    .unwrap();
    fixture.material.result_sources.clear();
    let original = from_v2(&fixture);
    let expected = page(original.case_id(), vec![original]);
    assert_eq!(
        list_result(MeasureDecisionReadQuery::default(), expected.clone()).unwrap(),
        expected
    );
}
