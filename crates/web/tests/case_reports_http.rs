mod case_report_http_support;
use application::case_reports::*;
use axum::http::StatusCode;
use case_report_http_support::*;
use serde_json::json;

#[tokio::test]
async fn request_returns_durable_accepted_identity_filters_and_safe_state() {
    let (workflow, router) = setup(false, None);
    let (status, headers, bytes) = send(router, "POST", "", Some(request_body()), true).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    private(&headers);
    let value = body(&bytes);
    assert_eq!(value["id"], ID);
    assert_eq!(value["scope"], "office");
    assert_eq!(value["state"], "queued");
    assert_eq!(value["filters"], request_body()["filters"]);
    assert_eq!(value["operation_id"], ID);
    assert_eq!(value["requested_at"], "2026-09-27T00:00:00Z");
    assert_eq!(value["notice"], json!(null));
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Request(report(false).command)]
    );
    let text = String::from_utf8(bytes).unwrap();
    for secret in [
        "auth_generation",
        "account_revision",
        "report-session",
        "lease",
        "wrapped_dek",
    ] {
        assert!(!text.contains(secret));
    }
}
#[tokio::test]
async fn terminal_replay_returns_ready_metadata_without_starting_another_operation() {
    let (workflow, router) = setup(true, None);
    let (status, headers, bytes) = send(router, "POST", "", Some(request_body()), true).await;
    assert_eq!(status, StatusCode::OK);
    private(&headers);
    let value = body(&bytes);
    assert_eq!(value["state"], "ready");
    assert_eq!(value["ready"]["snapshot_digest"], "02".repeat(32));
    assert_eq!(value["ready"]["artifacts"].as_array().unwrap().len(), 2);
    assert_eq!(value["notice"]["read_at"], json!(null));
    assert_eq!(workflow.calls.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn owned_list_passes_cursor_limit_and_unread_filter_without_acknowledging() {
    let (workflow, router) = setup(true, None);
    let (status, headers, bytes) = send(
        router,
        "GET",
        &format!("?limit=7&after_id={ID}&unread_only=true"),
        None,
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    private(&headers);
    let value = body(&bytes);
    assert_eq!(value["checked_at"], "2026-09-27T00:00:00Z");
    assert_eq!(value["reports"][0]["id"], ID);
    assert_eq!(value["has_more"], false);
    assert_eq!(value["next_after_id"], json!(null));
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::List(CaseReportQuery {
            limit: 7,
            after_id: Some(report(true).id),
            unread_only: true
        })]
    );
}
#[tokio::test]
async fn detail_does_not_acknowledge_and_notice_read_is_an_explicit_operation() {
    let (workflow, router) = setup(true, None);
    let (_, headers, bytes) = send(router.clone(), "GET", &format!("/{ID}"), None, true).await;
    private(&headers);
    assert_eq!(body(&bytes)["notice"]["read_at"], json!(null));
    let (status, headers, bytes) = send(
        router,
        "POST",
        &format!("/{ID}/notice-read"),
        Some(json!({})),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    private(&headers);
    assert_eq!(body(&bytes)["notice"]["read_at"], "2026-09-27T00:00:00Z");
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Get(report(true).id), Call::Ack(report(true).id)]
    );
}
#[tokio::test]
async fn downloads_keep_exact_bytes_attachment_headers_and_do_not_acknowledge() {
    for (format, content_type, expected, kind) in [
        ("pdf", "application/pdf", pdf(), CaseReportFormat::Pdf),
        (
            "csv",
            "text/csv; charset=utf-8",
            csv(),
            CaseReportFormat::Csv,
        ),
    ] {
        let (workflow, router) = setup(true, None);
        let (status, headers, bytes) = send(
            router,
            "GET",
            &format!("/{ID}/download?format={format}"),
            None,
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        private(&headers);
        assert_eq!(bytes, expected);
        assert_eq!(headers["content-type"], content_type);
        assert_eq!(headers["x-content-type-options"], "nosniff");
        assert_eq!(
            headers["content-disposition"],
            format!("attachment; filename=\"report-{ID}.{format}\"")
        );
        assert_eq!(headers["content-length"], bytes.len().to_string());
        assert_eq!(headers["x-report-snapshot-digest"], "02".repeat(32));
        assert_eq!(
            *workflow.calls.lock().unwrap(),
            vec![Call::Download(report(true).id, kind)]
        );
    }
}
#[tokio::test]
async fn every_route_requires_identity_before_touching_the_workflow() {
    for (method, path, payload) in [
        ("POST", String::new(), Some(request_body())),
        ("GET", String::new(), None),
        ("GET", format!("/{ID}"), None),
        ("GET", format!("/{ID}/download?format=pdf"), None),
        ("POST", format!("/{ID}/notice-read"), Some(json!({}))),
    ] {
        let (workflow, router) = setup(true, None);
        let (status, headers, _) = send(router, method, &path, payload, false).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        private(&headers);
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn malformed_or_client_supplied_scope_identity_and_filters_never_reach_store() {
    for mutation in 0..8 {
        let mut value = request_body();
        match mutation {
            0 => value["scope"] = json!("office"),
            1 => value["requester_id"] = json!(ID),
            2 => value["filters"]["unknown"] = json!(1),
            3 => value["filters"]["status"] = json!("all_private"),
            4 => value["operation_id"] = json!("00000000-0000-0000-0000-000000000000"),
            5 => value["filters"]["created_before"] = json!("2028-01-01T00:00:00Z"),
            6 => value["filters"]["created_from"] = json!("2026-09-01T00:00:00-06:00"),
            _ => value["filters"]["assigned_litigator"] = json!("not-a-uuid"),
        }
        let (workflow, router) = setup(false, None);
        let (status, headers, _) = send(router, "POST", "", Some(value), true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "mutation {mutation}");
        private(&headers);
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn invalid_queries_paths_and_notice_bodies_do_not_start_work() {
    let paths = [
        "?limit=0".into(),
        "?limit=101".into(),
        "?scope=office".into(),
        "?unread_only=1".into(),
        "?limit=1&limit=2".into(),
        format!("/{ID}?scope=office"),
        "/not-a-uuid".into(),
        format!("/{ID}/download?format=html"),
        format!("/{ID}/download?format=pdf&path=secret"),
    ];
    for path in paths {
        let (workflow, router) = setup(true, None);
        let (status, headers, _) = send(router, "GET", &path, None, true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}");
        private(&headers);
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    let (workflow, router) = setup(true, None);
    let (status, headers, _) = send(
        router,
        "POST",
        &format!("/{ID}/notice-read"),
        Some(json!({"read_at":"2020-01-01"})),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    private(&headers);
    assert!(workflow.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn typed_failures_have_stable_status_and_never_expose_private_diagnostics() {
    for (failure, status) in [
        ("not_found", 404),
        ("operation_conflict", 409),
        ("not_ready", 409),
        ("capacity_exceeded", 422),
        ("access_revoked", 403),
        ("render_unavailable", 503),
        ("render_failed", 500),
        ("stored_inconsistent", 500),
        ("denied", 403),
        ("port", 500),
    ] {
        let (_, router) = setup(true, Some(failure));
        let (actual, headers, bytes) = send(router, "GET", &format!("/{ID}"), None, true).await;
        assert_eq!(actual.as_u16(), status, "{failure}");
        private(&headers);
        let value = body(&bytes);
        assert!(value["error"]["code"].is_string());
        assert!(!String::from_utf8(bytes)
            .unwrap()
            .contains("private database payload"));
    }
}
#[tokio::test]
async fn oversized_request_is_rejected_before_workflow_execution() {
    let (workflow, router) = setup(false, None);
    let mut value = request_body();
    value["padding"] = json!("x".repeat(8192));
    let (status, headers, _) = send(router, "POST", "", Some(value), true).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    private(&headers);
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn litigators_default_page_projects_only_public_authorized_choices() {
    let (workflow, router) = setup(false, None);
    let (status, headers, bytes) = send(router, "GET", "/litigators", None, true).await;
    assert_eq!(status, StatusCode::OK);
    private(&headers);
    assert_eq!(
        body(&bytes),
        json!({
            "scope":"office", "checked_at":"2026-09-27T00:00:00Z",
            "litigators":[{"user_id":ID,"email":"lawyer@example.test"}],
            "has_more":false,"next_after_id":null
        })
    );
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Litigators(CaseReportLitigatorQuery {
            kind: CaseReportKind::CaseState,
            limit: 20,
            after_id: None
        })]
    );
}

#[tokio::test]
async fn litigators_exact_cursor_and_limit_reach_the_workflow() {
    let (workflow, router) = setup(false, None);
    let after = "00000000-0000-0000-0000-000000000008";
    let path = format!("/litigators?limit=1&after_id={after}");
    let (status, _, bytes) = send(router, "GET", &path, None, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body(&bytes)["has_more"], true);
    assert_eq!(body(&bytes)["next_after_id"], ID);
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Litigators(CaseReportLitigatorQuery {
            kind: CaseReportKind::CaseState,
            limit: 1,
            after_id: Some(domain::identity::UserId::from_uuid(
                uuid::Uuid::parse_str(after).unwrap()
            )),
        })]
    );
}

#[tokio::test]
async fn litigators_reject_invalid_unknown_duplicate_and_nil_queries_before_the_workflow() {
    for query in [
        "limit=0",
        "limit=101",
        "limit=-1",
        "limit=1.5",
        "limit=1&limit=2",
        "scope=office",
        "unread_only=true",
        "after_id=bad",
        "after_id=",
        "after_id=00000000-0000-0000-0000-000000000000",
    ] {
        let (workflow, router) = setup(false, None);
        let (status, headers, _) =
            send(router, "GET", &format!("/litigators?{query}"), None, true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
        private(&headers);
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn litigators_require_bearer_identity_and_preserve_permission_denial() {
    let (workflow, router) = setup(false, None);
    let (status, _, _) = send(router, "GET", "/litigators", None, false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(workflow.calls.lock().unwrap().is_empty());
    let (workflow, router) = setup(false, Some("denied"));
    let (status, headers, bytes) = send(router, "GET", "/litigators", None, true).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    private(&headers);
    assert_eq!(body(&bytes)["error"]["code"], "permission_denied");
    assert_eq!(workflow.calls.lock().unwrap().len(), 1);
}
