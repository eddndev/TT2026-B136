mod document_support;

use std::sync::{atomic::Ordering, Arc};

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use document_support::{StubIdentity, StubWorkflow, CASE_UUID, DOCUMENT_UUID};
use serde_json::{json, Value};
use tower::ServiceExt;

fn base() -> String {
    format!("/api/v1/cases/{CASE_UUID}/documents/{DOCUMENT_UUID}/metadata")
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
                .header("content-type", "application/json")
                .body(body.into())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

fn replacement(expected: u32) -> Value {
    json!({"expected_metadata_revision":expected,"document_type":" Escrito ",
        "classification":" Penal ","tags":[" z ","a,b","acci\u{00f3}n","a,b"]})
}

#[tokio::test]
async fn current_metadata_has_its_own_revision_and_case_document_identity() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = request(
        &workflow,
        Method::GET,
        &base(),
        Some("owner-token"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["case_id"], CASE_UUID.to_string());
    assert_eq!(body["id"], DOCUMENT_UUID.to_string());
    assert_eq!(body["metadata_revision"], 3);
    assert_eq!(body["document_type"], "Escrito");
    assert_eq!(body["classification"], "Penal");
    assert!(body["tags"].is_array());
    for key in ["version", "vault", "evidence", "changed_at", "changed_by"] {
        assert!(body.get(key).is_none(), "unexpected metadata field {key}");
    }
}

#[tokio::test]
async fn metadata_replacement_passes_normalized_individual_values_and_expected_revision() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = request(
        &workflow,
        Method::PUT,
        &base(),
        Some("owner-token"),
        replacement(3).to_string(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["metadata_revision"], 4);
    assert_eq!(body["document_type"], "Escrito");
    assert_eq!(body["classification"], "Penal");
    assert_eq!(body["tags"], json!(["a,b", "acci\u{00f3}n", "z"]));
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn clearing_metadata_records_an_empty_successor_instead_of_revision_zero() {
    let workflow = Arc::new(StubWorkflow::default());
    let body =
        json!({"expected_metadata_revision":3,"document_type":null,"classification":" ","tags":[]});
    let response = request(
        &workflow,
        Method::PUT,
        &base(),
        Some("owner-token"),
        body.to_string(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["metadata_revision"], 4);
    assert!(body["document_type"].is_null());
    assert!(body["classification"].is_null());
    assert_eq!(body["tags"], json!([]));
}

#[tokio::test]
async fn metadata_history_preserves_the_author_and_uses_a_descending_exclusive_cursor() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = request(
        &workflow,
        Method::GET,
        &format!("{}/history?limit=1&before_revision=3", base()),
        Some("owner-token"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["case_id"], CASE_UUID.to_string());
    assert_eq!(body["id"], DOCUMENT_UUID.to_string());
    assert_eq!(body["revisions"].as_array().unwrap().len(), 1);
    assert_eq!(body["revisions"][0]["metadata_revision"], 2);
    assert_eq!(
        body["revisions"][0]["changed_by"]["email"],
        "historical@example.com"
    );
    assert!(body["revisions"][0]["changed_by"]["id"].is_string());
    assert!(body["revisions"][0]["changed_at"]
        .as_str()
        .unwrap()
        .ends_with('Z'));
    assert_eq!(
        body["revisions"][0]["metadata_digest"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(body["has_more"], true);
    assert_eq!(body["next_before_revision"], 2);
}

#[tokio::test]
async fn metadata_actions_require_sessions_and_preserve_client_denial() {
    let workflow = Arc::new(StubWorkflow::default());
    for (method, path, body) in [
        (Method::GET, base(), String::new()),
        (Method::PUT, base(), replacement(3).to_string()),
        (Method::GET, format!("{}/history", base()), String::new()),
    ] {
        for (token, status) in [
            (None, StatusCode::UNAUTHORIZED),
            (Some("expired-token"), StatusCode::UNAUTHORIZED),
            (Some("client-token"), StatusCode::FORBIDDEN),
        ] {
            let response = request(&workflow, method.clone(), &path, token, body.clone()).await;
            assert_eq!(response.status(), status, "{method} {path}");
        }
    }
}

#[tokio::test]
async fn metadata_conflict_and_exhaustion_are_distinct_and_authorization_comes_first() {
    let workflow = Arc::new(StubWorkflow::default());
    for (expected, code) in [
        (2, "document_metadata_conflict"),
        (u32::MAX, "document_metadata_revision_exhausted"),
    ] {
        let response = request(
            &workflow,
            Method::PUT,
            &base(),
            Some("owner-token"),
            replacement(expected).to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(json_body(response).await["error"]["code"], code);
        let response = request(
            &workflow,
            Method::PUT,
            &base(),
            Some("client-token"),
            replacement(expected).to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
async fn foreign_and_missing_metadata_resources_are_indistinguishable() {
    let workflow = Arc::new(StubWorkflow::default());
    let mut errors = Vec::new();
    for path in [
        base().replace(&CASE_UUID.to_string(), &uuid::Uuid::nil().to_string()),
        base().replace(&DOCUMENT_UUID.to_string(), &uuid::Uuid::nil().to_string()),
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
        errors.push(json_body(response).await);
    }
    assert_eq!(errors[0], errors[1]);
    assert_eq!(errors[0]["error"]["code"], "document_not_found");
}

#[tokio::test]
async fn invalid_metadata_json_and_unknown_fields_never_reach_the_workflow() {
    let workflow = Arc::new(StubWorkflow::default());
    for body in [
        "{",
        "null",
        "[]",
        "{}",
        "{\"tags\":[]}",
        "{\"expected_metadata_revision\":-1,\"tags\":[]}",
        "{\"expected_metadata_revision\":4294967296,\"tags\":[]}",
        "{\"expected_metadata_revision\":3,\"tags\":\"a,b\"}",
        "{\"expected_metadata_revision\":3,\"tags\":[],\"changed_by\":\"forged\"}",
        "{\"expected_metadata_revision\":3,\"tags\":[],\"tags\":[\"duplicate-key\"]}",
    ] {
        let response = request(&workflow, Method::PUT, &base(), Some("owner-token"), body).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{body}");
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn semantic_metadata_errors_and_page_bounds_are_rejected_before_use_cases() {
    let workflow = Arc::new(StubWorkflow::default());
    for (field, value) in [
        ("document_type", json!("x".repeat(81))),
        ("classification", json!("\nPenal")),
        ("tags", json!([""])),
        ("tags", json!(["x".repeat(41)])),
        ("tags", json!(vec!["same"; 21])),
        ("tags", json!(["\u{0085}label"])),
    ] {
        let mut body = replacement(3);
        body[field] = value;
        let response = request(
            &workflow,
            Method::PUT,
            &base(),
            Some("owner-token"),
            body.to_string(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            json_body(response).await["error"]["code"],
            "invalid_document_metadata"
        );
    }
    for (query, status) in [
        ("limit=0", StatusCode::UNPROCESSABLE_ENTITY),
        ("limit=101", StatusCode::UNPROCESSABLE_ENTITY),
        ("before_revision=0", StatusCode::UNPROCESSABLE_ENTITY),
        ("before_revision=-1", StatusCode::BAD_REQUEST),
        ("offset=1", StatusCode::BAD_REQUEST),
    ] {
        let response = request(
            &workflow,
            Method::GET,
            &format!("{}/history?{query}", base()),
            Some("owner-token"),
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), status, "{query}");
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn metadata_json_is_bounded_independently_of_the_document_file_limit() {
    let workflow = Arc::new(StubWorkflow::default());
    let mut body = replacement(3).to_string();
    body.extend(std::iter::repeat_n(' ', 8192 - body.len()));
    let response = request(
        &workflow,
        Method::PUT,
        &base(),
        Some("owner-token"),
        body.clone(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let calls = workflow.calls.load(Ordering::SeqCst);
    body.push(' ');
    let response = request(&workflow, Method::PUT, &base(), Some("owner-token"), body).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), calls);
}
