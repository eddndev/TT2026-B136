use super::*;

#[tokio::test]
async fn list_queries_reject_unknown_duplicate_noncanonical_and_out_of_range_values() {
    let cursor = initial(1).reference.id();
    for suffix in [
        "limit=0".to_string(),
        "limit=21".into(),
        "limit=01".into(),
        "limit=-1".into(),
        "limit=1.0".into(),
        "limit=1e1".into(),
        "limit=99999999999".into(),
        "limit=1&limit=2".into(),
        "after_operation_id=anything".into(),
        "active=true".into(),
        "after_id=not-a-uuid".into(),
        format!("after_id={cursor}&after_id={cursor}"),
    ] {
        let (status, body) = request(MockRecords::new(), &format!("{}?{suffix}", base())).await;
        assert_eq!(status, 400, "{suffix}: {body}");
    }
}

#[tokio::test]
async fn exact_route_requires_one_canonical_digest_and_rejects_extra_query_keys() {
    let selected = initial(1).reference;
    let path = format!(
        "{}/revisions/{}",
        current_path(selected.id()),
        selected.revision().get()
    );
    for suffix in [
        String::new(),
        "?capture_digest=".into(),
        "?capture_digest=00".into(),
        format!("?capture_digest={}", "AA".repeat(32)),
        format!(
            "?capture_digest={0}&capture_digest={0}",
            selected.digest().to_hex()
        ),
        format!("?capture_digest={}&limit=1", selected.digest().to_hex()),
    ] {
        let (status, body) = request(MockRecords::new(), &format!("{path}{suffix}")).await;
        assert_eq!(status, 400, "{suffix}: {body}");
    }
    for revision in ["0", "01", "-1", "4294967296"] {
        let path = format!(
            "{}/revisions/{revision}?capture_digest={}",
            current_path(selected.id()),
            selected.digest().to_hex()
        );
        let (status, body) = request(MockRecords::new(), &path).await;
        assert_eq!(status, 400, "{body}");
    }
    let (status, body) = request(
        MockRecords::new(),
        &format!(
            "{}?capture_digest={}",
            current_path(selected.id()),
            selected.digest().to_hex()
        ),
    )
    .await;
    assert_eq!(status, 400, "{body}");
}

#[tokio::test]
async fn path_identifiers_must_be_canonical_uuid_before_workflow_invocation() {
    let id = "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA";
    for path in [
        format!("{}/{id}", base()),
        format!("/api/v1/cases/{id}/measures"),
        format!("{}/not-a-uuid", base()),
    ] {
        let (status, body) = request(MockRecords::new(), &path).await;
        assert_eq!(status, 400, "{body}");
    }
}

#[tokio::test]
async fn missing_bearer_rejects_before_any_read_and_preserves_no_store() {
    let (status, body) = raw(MockRecords::new(), &base(), None).await;
    assert_eq!(status, 401, "{body}");
}

#[tokio::test]
async fn authorization_absence_budget_and_integrity_errors_keep_their_status_without_details() {
    let selected = initial(1).reference;
    let errors = [
        (ApplicationError::InvalidSession, 401),
        (ApplicationError::PermissionDenied, 403),
        (
            ApplicationError::MeasureRecordRead(MeasureRecordReadError::NotFound),
            404,
        ),
        (
            ApplicationError::MeasureRecordRead(MeasureRecordReadError::IncompleteHistory),
            422,
        ),
        (
            ApplicationError::MeasureRecordRead(MeasureRecordReadError::StoredInconsistent(
                "private row or source detail".into(),
            )),
            500,
        ),
    ];
    for (error, expected) in errors {
        let mut port = MockRecords::new();
        port.expect_get()
            .times(1)
            .return_once(move |token, case, id| {
                assert_eq!((token, case, id), ("staff-token", case_id(), selected.id()));
                Err(error)
            });
        let (status, body) = request(port, &current_path(selected.id())).await;
        assert_eq!(status, expected, "{body}");
        if status == 500 {
            assert_eq!(body, internal());
        }
    }
}
