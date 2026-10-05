use super::*;

fn pair() -> (
    MeasureAdministrativeStoredOperation,
    MeasureAdministrativeStoredOperation,
) {
    let fixture = crate::record_support::RecordFixture::initial();
    let first = stored(&fixture);
    let second = stored(&crate::record_support::RecordFixture::next(
        &first.capture,
        &fixture.history,
        1,
    ));
    (first, second)
}
fn page(items: Vec<MeasureAdministrativeStoredOperation>) -> MeasureAdministrativePage {
    MeasureAdministrativePage {
        case_id: case_id(),
        items,
        has_more: false,
        next_after_operation_id: None,
    }
}

#[tokio::test]
async fn administrative_page_uses_default_and_exclusive_operation_cursors() {
    let (first, second) = pair();
    let mut read = MockRead::new();
    let returned = first.clone();
    read.expect_list()
        .times(1)
        .return_once(move |token, case, query| {
            assert_eq!(token, "staff-token");
            assert_eq!(case, case_id());
            assert_eq!(query, MeasureAdministrativeReadQuery::default());
            Ok(page(vec![returned]))
        });
    let (status, body) = request(MockWrite::new(), read, "GET", &base(), None).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["case_id"], case_id().to_string());
    assert_stored(&body["items"][0], &first);
    assert_eq!(body["has_more"], false);
    assert!(body["next_after_operation_id"].is_null());

    let mut read = MockRead::new();
    let expected = second.clone();
    let after = first.origin.operation_id;
    read.expect_list().times(1).return_once(move |_, _, query| {
        assert_eq!(
            query,
            MeasureAdministrativeReadQuery::new(1, Some(after)).unwrap()
        );
        Ok(MeasureAdministrativePage {
            case_id: case_id(),
            items: vec![expected.clone()],
            has_more: true,
            next_after_operation_id: Some(expected.origin.operation_id),
        })
    });
    let (status, body) = request(
        MockWrite::new(),
        read,
        "GET",
        &format!("{}?limit=1&after_operation_id={after}", base()),
        None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_stored(&body["items"][0], &second);
    assert_eq!(
        body["next_after_operation_id"],
        second.origin.operation_id.to_string()
    );
}

#[tokio::test]
async fn page_rejects_scope_order_duplicates_cursor_size_and_false_continuations() {
    let (first, second) = pair();
    for kind in 0..9 {
        let mut returned = page(vec![first.clone(), second.clone()]);
        let mut path = format!("{}?limit=2", base());
        match kind {
            0 => returned.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(99)),
            1 => returned.items.reverse(),
            2 => returned.items[1] = first.clone(),
            3 => {
                path = format!(
                    "{}?limit=2&after_operation_id={}",
                    base(),
                    first.origin.operation_id
                )
            }
            4 => path = format!("{}?limit=1", base()),
            5 => returned.next_after_operation_id = Some(second.origin.operation_id),
            6 => {
                returned.has_more = true;
                returned.items.pop();
                returned.next_after_operation_id = Some(first.origin.operation_id);
            }
            7 => {
                returned.has_more = true;
                returned.next_after_operation_id = Some(first.origin.operation_id);
            }
            _ => returned.items[1].origin.capture_digest = Sha256Digest::from_array([8; 32]),
        }
        let mut read = MockRead::new();
        read.expect_list()
            .times(1)
            .return_once(move |_, _, _| Ok(returned));
        let (status, body) = request(MockWrite::new(), read, "GET", &path, None).await;
        assert_eq!(status, 500, "{kind}: {body}");
        assert_eq!(body, internal());
    }
}

#[tokio::test]
async fn strict_list_and_operation_queries_are_rejected_before_ports() {
    let operation = correct();
    let mut paths = vec![];
    for query in [
        "limit=0",
        "limit=21",
        "limit=01",
        "limit=+1",
        "limit=1.0",
        "limit=1&limit=2",
        "after_id=00000000-0000-0000-0000-000000000001",
        "after_operation_id=bad",
        "extra=1",
    ] {
        paths.push(format!("{}?{query}", base()));
    }
    paths.push(format!("{}?limit=1", operation_path(&operation)));
    paths.push(format!("{}/bad", base()));
    for path in paths {
        let (status, body) = request(MockWrite::new(), MockRead::new(), "GET", &path, None).await;
        assert_eq!(status, 400, "{path}: {body}");
    }
}
