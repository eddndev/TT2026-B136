mod document_support;

use std::sync::{atomic::Ordering, Arc};

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use document_support::{StubIdentity, StubWorkflow, CASE_UUID, DOCUMENT_UUID};
use serde_json::Value;
use tower::ServiceExt;

fn base() -> String {
    format!("/api/v1/cases/{CASE_UUID}/documents/{DOCUMENT_UUID}/versions")
}

async fn request(
    workflow: &Arc<StubWorkflow>,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: impl Into<Body>,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    web::application_router(workflow.clone(), Arc::new(StubIdentity))
        .oneshot(
            request
                .header("x-document-name", "revised.txt")
                .body(body.into())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

#[tokio::test]
async fn append_preserves_identity_and_passes_the_expected_head_and_file() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = request(
        &workflow,
        Method::POST,
        &format!("{}?expected_version=3", base()),
        Some("owner-token"),
        "revised content",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = json(response).await;
    assert_eq!(body["case_id"], CASE_UUID.to_string());
    assert_eq!(body["id"], DOCUMENT_UUID.to_string());
    assert_eq!(body["version"], 4);
    assert_eq!(body["name"], "revised.txt");
    assert_eq!(body["sealed"], false);
    assert_eq!(body["current_metadata"]["metadata_revision"], 3);
}

#[tokio::test]
async fn history_passes_a_descending_cursor_and_returns_version_metadata() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = request(
        &workflow,
        Method::GET,
        &format!("{}?limit=1&before_version=3", base()),
        Some("owner-token"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["versions"].as_array().unwrap().len(), 1);
    assert_eq!(body["versions"][0]["version"], 2);
    assert_eq!(body["versions"][0]["case_id"], CASE_UUID.to_string());
    assert_eq!(body["has_more"], true);
    assert_eq!(body["next_before_version"], 2);
    assert_eq!(body["first_available_version"], 1);
    assert!(body["versions"][0].get("vault").is_none());
    assert!(body["versions"][0].get("evidence").is_none());
    assert!(body["versions"][0].get("current_metadata").is_none());
}

#[tokio::test]
async fn detail_and_actions_preserve_the_requested_snapshot() {
    let workflow = Arc::new(StubWorkflow::default());
    for (method, suffix) in [(Method::GET, "2"), (Method::POST, "2/seal")] {
        let response = request(
            &workflow,
            method,
            &format!("{}/{suffix}", base()),
            Some("owner-token"),
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json(response).await;
        assert_eq!(body["version"], 2);
        assert!(body.get("current_metadata").is_none());
    }
    let response = request(
        &workflow,
        Method::POST,
        &format!("{}/2/verify", base()),
        Some("owner-token"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["id"], DOCUMENT_UUID.to_string());
    assert_eq!(body["version"], 2);
    assert_eq!(body["verdict"], "valid");
    assert_eq!(body["document_digest"], "02".repeat(32));
}

#[tokio::test]
async fn evidence_response_identifies_the_exact_version_without_changing_zip_bytes() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = request(
        &workflow,
        Method::GET,
        &format!("{}/2/evidence", base()),
        Some("owner-token"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/zip");
    assert_eq!(
        response.headers()["x-document-id"],
        DOCUMENT_UUID.to_string()
    );
    assert_eq!(response.headers()["x-document-version"], "2");
    assert_eq!(response.headers()["x-document-digest"], "02".repeat(32));
    assert_eq!(
        &to_bytes(response.into_body(), 1024).await.unwrap()[..],
        b"historical zip bytes"
    );
}

#[tokio::test]
async fn version_queries_reject_invalid_or_unknown_input_before_use_cases() {
    let workflow = Arc::new(StubWorkflow::default());
    for (method, suffix, expected) in [
        (Method::GET, "?limit=0", StatusCode::UNPROCESSABLE_ENTITY),
        (Method::GET, "?limit=101", StatusCode::UNPROCESSABLE_ENTITY),
        (
            Method::GET,
            "?before_version=0",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (Method::GET, "?before_version=-1", StatusCode::BAD_REQUEST),
        (Method::GET, "?offset=1", StatusCode::BAD_REQUEST),
        (Method::POST, "", StatusCode::BAD_REQUEST),
        (
            Method::POST,
            "?expected_version=0",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            Method::POST,
            "?expected_version=4294967296",
            StatusCode::BAD_REQUEST,
        ),
        (
            Method::POST,
            "?expected_version=1&unknown=1",
            StatusCode::BAD_REQUEST,
        ),
        (Method::GET, "/0", StatusCode::BAD_REQUEST),
        (Method::POST, "/invalid/seal", StatusCode::BAD_REQUEST),
        (Method::GET, "/4294967296/evidence", StatusCode::BAD_REQUEST),
    ] {
        let response = request(
            &workflow,
            method,
            &format!("{}{suffix}", base()),
            Some("owner-token"),
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), expected, "{suffix}");
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn all_version_endpoints_require_authentication_and_preserve_client_denial() {
    let workflow = Arc::new(StubWorkflow::default());
    for (method, suffix) in [
        (Method::GET, ""),
        (Method::POST, "?expected_version=3"),
        (Method::GET, "/2"),
        (Method::POST, "/2/seal"),
        (Method::POST, "/2/verify"),
        (Method::GET, "/2/evidence"),
    ] {
        for (token, status) in [
            (None, StatusCode::UNAUTHORIZED),
            (Some("expired-token"), StatusCode::UNAUTHORIZED),
            (Some("client-token"), StatusCode::FORBIDDEN),
        ] {
            let response = request(
                &workflow,
                method.clone(),
                &format!("{}{suffix}", base()),
                token,
                "revised content",
            )
            .await;
            assert_eq!(response.status(), status, "{suffix}");
        }
    }
}

#[tokio::test]
async fn append_conflict_and_version_exhaustion_have_actionable_error_codes() {
    let workflow = Arc::new(StubWorkflow::default());
    for (version, code) in [
        (2, "document_version_conflict"),
        (u32::MAX, "document_version_exhausted"),
    ] {
        let response = request(
            &workflow,
            Method::POST,
            &format!("{}?expected_version={version}", base()),
            Some("owner-token"),
            "revised content",
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(json(response).await["error"]["code"], code);
    }
}

#[tokio::test]
async fn absent_versions_and_foreign_associations_have_the_same_not_found_response() {
    let workflow = Arc::new(StubWorkflow::default());
    let mut errors = Vec::new();
    for path in [
        format!("{}/9", base()),
        format!("{}/2", base()).replace(&CASE_UUID.to_string(), &uuid::Uuid::nil().to_string()),
        format!("{}/2", base()).replace(&DOCUMENT_UUID.to_string(), &uuid::Uuid::nil().to_string()),
    ] {
        let response = request(
            &workflow,
            Method::GET,
            &path,
            Some("owner-token"),
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        errors.push(json(response).await);
    }
    assert_eq!(errors[0], errors[1]);
    assert_eq!(errors[0], errors[2]);
}

#[tokio::test]
async fn version_append_obeys_the_document_size_limit() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = request(
        &workflow,
        Method::POST,
        &format!("{}?expected_version=3", base()),
        Some("owner-token"),
        vec![0; 16 * 1024 * 1024 + 1],
    )
    .await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}
