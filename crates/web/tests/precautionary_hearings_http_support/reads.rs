use super::*;

#[tokio::test]
async fn list_current_exact_and_operation_reads_preserve_requested_selectors() {
    let original = initial();
    let row = cancelled();
    let id = row.capture.review.command.hearing_id;
    let mut read = MockRead::new();
    let returned = row.clone();
    read.expect_list()
        .times(1)
        .return_once(move |token, case, query| {
            assert_eq!((token, case), ("staff-token", case_id()));
            assert_eq!(query.limit(), 1);
            assert_eq!(
                query.after_id().unwrap().as_uuid(),
                uuid::Uuid::from_u128(1)
            );
            Ok(PrecautionaryHearingRecordPage {
                case_id: case,
                items: vec![returned],
                has_more: true,
                next_after_id: Some(id),
            })
        });
    let (status, value) = request(
        MockContext::new(),
        MockWrite::new(),
        read,
        "GET",
        &format!(
            "{}?limit=1&after_id=00000000-0000-0000-0000-000000000001",
            base()
        ),
        None,
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(
        value["items"][0]["capture"]["review"]["status"],
        "cancelled"
    );
    assert_eq!(value["next_after_id"], id.to_string());
    assert_eq!(value["has_more"], true);
    for revision in [None, Some(PrecautionaryHearingRevision::initial())] {
        let returned = if revision.is_some() {
            original.clone()
        } else {
            row.clone()
        };
        let expected = returned.capture.capture_digest.to_hex();
        let mut read = MockRead::new();
        read.expect_get()
            .times(1)
            .return_once(move |token, case, hearing, exact| {
                assert_eq!(
                    (token, case, hearing, exact),
                    ("staff-token", case_id(), id, revision)
                );
                Ok(returned)
            });
        let path = format!(
            "{}/{}{}",
            base(),
            id,
            if revision.is_some() {
                "/revisions/1"
            } else {
                ""
            }
        );
        let (status, value) = request(
            MockContext::new(),
            MockWrite::new(),
            read,
            "GET",
            &path,
            None,
        )
        .await;
        assert_eq!(status, 200, "{value}");
        assert_eq!(value["capture"]["capture_digest"], expected);
    }
    let op = original.capture.review.command.operation_id;
    let mut read = MockRead::new();
    read.expect_get_operation()
        .times(1)
        .return_once(move |token, case, operation| {
            assert_eq!((token, case, operation), ("staff-token", case_id(), op));
            Ok(original)
        });
    let (status, value) = request(
        MockContext::new(),
        MockWrite::new(),
        read,
        "GET",
        &format!("{}/operations/{op}", base()),
        None,
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["capture"]["review"]["result_revision"], 1);
    assert_eq!(value["history"]["captures"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn unsupported_queries_and_noncanonical_paths_never_call_the_reader() {
    for suffix in [
        "?limit=0",
        "?limit=21",
        "?limit=01",
        "?limit=1&limit=2",
        "?status=scheduled",
        "?after_id=00000000000000000000000000000001",
    ] {
        let (status, value) = request(
            MockContext::new(),
            MockWrite::new(),
            MockRead::new(),
            "GET",
            &format!("{}{suffix}", base()),
            None,
        )
        .await;
        assert_eq!(status, 400, "{suffix}: {value}");
    }
    let id = initial().capture.review.command.hearing_id;
    for suffix in [
        "/revisions/0",
        "/revisions/01",
        "/revisions/4294967296",
        "?limit=1",
    ] {
        let (status, value) = request(
            MockContext::new(),
            MockWrite::new(),
            MockRead::new(),
            "GET",
            &format!("{}/{id}{suffix}", base()),
            None,
        )
        .await;
        assert_eq!(status, 400, "{suffix}: {value}");
    }
}

#[tokio::test]
async fn response_scope_exact_revision_and_page_continuation_cannot_be_substituted() {
    for fault in 0..6 {
        let row = initial();
        let id = row.capture.review.command.hearing_id;
        let mut page = PrecautionaryHearingRecordPage {
            case_id: case_id(),
            items: vec![row.clone()],
            has_more: false,
            next_after_id: None,
        };
        match fault {
            0 => page.case_id = CaseId::new(),
            1 => page.items.push(row),
            2 => page.next_after_id = Some(id),
            3 => page.has_more = true,
            4 => page.items[0].capture.review.case_id = CaseId::new(),
            _ => page.items[0].history.captures = vec![page.items[0].capture.clone(); 257],
        }
        let mut read = MockRead::new();
        read.expect_list()
            .times(1)
            .return_once(move |_, _, _| Ok(page));
        let result = request(
            MockContext::new(),
            MockWrite::new(),
            read,
            "GET",
            &base(),
            None,
        )
        .await;
        assert_eq!(result, (500, internal()), "fault {fault}");
    }
    let row = cancelled();
    let id = row.capture.review.command.hearing_id;
    let mut read = MockRead::new();
    read.expect_get()
        .times(1)
        .return_once(move |_, _, _, _| Ok(row));
    let result = request(
        MockContext::new(),
        MockWrite::new(),
        read,
        "GET",
        &format!("{}/{id}/revisions/1", base()),
        None,
    )
    .await;
    assert_eq!(result, (500, internal()));
}
