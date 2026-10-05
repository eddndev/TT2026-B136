use super::*;

async fn list(query: MeasureRecordReadQuery, page: MeasureRecordPage) -> (u16, Value) {
    let mut port = MockRecords::new();
    port.expect_list()
        .times(1)
        .return_once(move |token, case, q| {
            assert_eq!((token, case, q), ("staff-token", case_id(), query));
            Ok(page)
        });
    let mut path = format!("{}?limit={}", base(), query.limit());
    if let Some(id) = query.after_id() {
        path.push_str(&format!("&after_id={id}"));
    }
    request(port, &path).await
}

#[tokio::test]
async fn list_propagates_exclusive_cursor_and_returns_actual_marked_head() {
    let prior = initial(1).reference.id();
    let first = initial(2);
    let second = administrative(&initial(3), 3, true);
    let last = second.reference.id();
    let mut returned = page(vec![first, second]);
    returned.has_more = true;
    returned.next_after_id = Some(last);
    let (status, body) = list(
        MeasureRecordReadQuery::new(2, Some(prior)).unwrap(),
        returned,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["case_id"], case_id().to_string());
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
    assert_eq!(body["items"][1]["validity"], "entered_in_error");
    assert_eq!(body["has_more"], true);
    assert_eq!(body["next_after_id"], last.to_string());
}

#[tokio::test]
async fn empty_default_page_has_no_invented_continuation() {
    let mut port = MockRecords::new();
    port.expect_list()
        .times(1)
        .return_once(|token, case, query| {
            assert_eq!(
                (token, case, query),
                ("staff-token", case_id(), MeasureRecordReadQuery::default())
            );
            Ok(page(vec![]))
        });
    let (status, body) = request(port, &base()).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({"case_id":case_id().to_string(),"items":[],"has_more":false,"next_after_id":null})
    );
}

#[tokio::test]
async fn inconsistent_page_scope_order_size_and_continuation_fail_closed() {
    let first = initial(1);
    let second = initial(2);
    let query = MeasureRecordReadQuery::new(2, None).unwrap();
    let valid = page(vec![first.clone(), second.clone()]);
    let mut cases = Vec::new();
    let mut wrong = valid.clone();
    wrong.case_id = CaseId::from_uuid(Uuid::from_u128(999));
    cases.push(wrong);
    let mut wrong = valid.clone();
    wrong.items.reverse();
    cases.push(wrong);
    cases.push(page(vec![first.clone(), first.clone()]));
    cases.push(page(vec![first.clone(), second.clone(), initial(3)]));
    let mut wrong = valid.clone();
    wrong.has_more = true;
    cases.push(wrong);
    let mut wrong = valid.clone();
    wrong.has_more = true;
    wrong.next_after_id = Some(first.reference.id());
    cases.push(wrong);
    let mut wrong = valid;
    wrong.next_after_id = Some(second.reference.id());
    cases.push(wrong);
    let mut wrong = page(vec![first.clone()]);
    wrong.has_more = true;
    wrong.next_after_id = Some(first.reference.id());
    cases.push(wrong);
    for returned in cases {
        let (status, body) = list(query, returned).await;
        assert_eq!(status, 500, "{body}");
        assert_eq!(body, internal());
    }
    let query = MeasureRecordReadQuery::new(2, Some(first.reference.id())).unwrap();
    let (status, body) = list(query, page(vec![first, second])).await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(body, internal());
}
