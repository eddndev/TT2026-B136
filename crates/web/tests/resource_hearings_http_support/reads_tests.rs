use super::*;

#[tokio::test]
async fn read_routes_preserve_scope_cursor_and_exact_revision_without_writes() {
    let mut read = MockRead::new();
    read.expect_list()
        .times(1)
        .return_once(|token, c, r, query| {
            assert_eq!((token, c, r), ("owner", case(), resource()));
            assert_eq!(query.limit(), 1);
            assert_eq!(
                query.after_id().unwrap().as_uuid(),
                uuid::Uuid::from_u128(1)
            );
            Ok(page())
        });
    let path = format!(
        "{}?limit=1&after_id=00000000-0000-0000-0000-000000000001",
        base()
    );
    let (status, value) = request(MockWrite::new(), read, "GET", &path, None).await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["case_id"], case().to_string());
    assert_eq!(value["resource_id"], resource().to_string());
    assert_eq!(value["items"][0]["hearing"]["id"], hearing_id().to_string());
    assert_eq!(value["has_more"], false);
    assert!(value["next_after_id"].is_null());
    for revision in [None, Some(ResourceHearingRevision::initial())] {
        let mut read = MockRead::new();
        read.expect_get()
            .times(1)
            .return_once(move |token, c, r, h, exact| {
                assert_eq!(
                    (token, c, r, h),
                    ("owner", case(), resource(), hearing_id())
                );
                assert_eq!(exact, revision);
                Ok(creation())
            });
        let path = format!(
            "{}/{}{}",
            base(),
            hearing_id(),
            if revision.is_some() {
                "/revisions/1"
            } else {
                ""
            }
        );
        let (status, value) = request(MockWrite::new(), read, "GET", &path, None).await;
        assert_eq!(status, 200, "{value}");
        assert_eq!(value["hearing"]["revision"], 1);
    }
}

#[tokio::test]
async fn read_queries_and_revision_paths_reject_unsupported_or_noncanonical_inputs() {
    for suffix in [
        "?limit=0",
        "?limit=21",
        "?limit=01",
        "?limit=1&limit=2",
        "?status=active",
        "?after_id=00000000000000000000000000000001",
    ] {
        let (status, value) = request(
            MockWrite::new(),
            MockRead::new(),
            "GET",
            &format!("{}{suffix}", base()),
            None,
        )
        .await;
        assert_eq!(status, 400, "{suffix}: {value}");
    }
    for suffix in [
        "/revisions/0",
        "/revisions/01",
        "/revisions/4294967296",
        "?limit=1",
    ] {
        let (status, value) = request(
            MockWrite::new(),
            MockRead::new(),
            "GET",
            &format!("{}/{}{suffix}", base(), hearing_id()),
            None,
        )
        .await;
        assert_eq!(status, 400, "{suffix}: {value}");
    }
}

#[tokio::test]
async fn list_envelope_and_continuation_must_match_the_requested_scope() {
    for fault in 0..4 {
        let mut result = page();
        match fault {
            0 => {
                result.items.clear();
                result.case_id = domain::cases::CaseId::new();
            }
            1 => result.items.push(creation()),
            2 => result.next_after_id = Some(hearing_id()),
            _ => {
                result.has_more = true;
                result.next_after_id = None;
            }
        }
        let mut read = MockRead::new();
        read.expect_list()
            .times(1)
            .return_once(move |_, _, _, _| Ok(result));
        let (status, value) = request(MockWrite::new(), read, "GET", &base(), None).await;
        assert_eq!((status, value), (500, internal()), "fault {fault}");
    }
}

#[tokio::test]
async fn workflow_errors_keep_authorization_absence_conflict_and_corruption_distinct() {
    for (error, expected) in [
        (ApplicationError::InvalidSession, 401),
        (ApplicationError::PermissionDenied, 403),
        (
            ApplicationError::ResourceActivity(ResourceActivityError::NotFound),
            404,
        ),
        (
            ApplicationError::ResourceActivity(ResourceActivityError::OperationConflict),
            409,
        ),
        (
            ApplicationError::ResourceActivity(ResourceActivityError::StoredInconsistent(
                "private database detail".into(),
            )),
            500,
        ),
    ] {
        let mut read = MockRead::new();
        read.expect_get()
            .times(1)
            .return_once(move |_, _, _, _, _| Err(error));
        let (status, value) = request(
            MockWrite::new(),
            read,
            "GET",
            &format!("{}/{}", base(), hearing_id()),
            None,
        )
        .await;
        assert_eq!(status, expected, "{value}");
        if expected == 500 {
            assert_eq!(value, internal());
        }
    }
}
