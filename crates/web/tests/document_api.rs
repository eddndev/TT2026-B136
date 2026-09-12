mod document_support;

use std::sync::{atomic::Ordering, Arc};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::response::Response;
use document_support::{StubIdentity, StubWorkflow, CASE_UUID, DOCUMENT_UUID};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

fn router(workflow: &Arc<StubWorkflow>) -> axum::Router {
    web::application_router(workflow.clone(), Arc::new(StubIdentity))
}

fn collection() -> String {
    format!("/api/v1/cases/{CASE_UUID}/documents")
}

fn document() -> String {
    format!("{}/{DOCUMENT_UUID}", collection())
}

fn upload_request(uri: &str) -> Request<Body> {
    Request::post(uri)
        .header("authorization", "Bearer owner-token")
        .header("x-document-name", "acta.txt")
        .body(Body::from("case document"))
        .unwrap()
}

async fn json(response: Response) -> Value {
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn uploading_requires_an_explicit_case_in_the_route() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = router(&workflow)
        .oneshot(upload_request(&collection()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = json(response).await;
    assert_eq!(body["case_id"], CASE_UUID.to_string());
    assert_eq!(body["id"], DOCUMENT_UUID.to_string());
    assert_eq!(body["version"], 1);
    assert_eq!(body["sealed"], false);
}

#[tokio::test]
async fn scoped_routes_pass_token_case_and_document_to_the_workflow() {
    let workflow = Arc::new(StubWorkflow::default());
    for operation in ["seal", "verify"] {
        let response = router(&workflow)
            .oneshot(
                Request::post(format!("{}/{operation}", document()))
                    .header("authorization", "Bearer owner-token")
                    .header("x-actor", "spoofed@example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json(response).await;
        if operation == "seal" {
            assert_eq!(body["case_id"], CASE_UUID.to_string());
            assert_eq!(body["sealed"], true);
        } else {
            assert_eq!(body["verdict"], "valid");
            assert_eq!(body["timestamp"]["status"], "passed");
        }
    }
    let evidence = router(&workflow)
        .oneshot(
            Request::get(format!("{}/evidence", document()))
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(evidence.status(), StatusCode::OK);
    assert_eq!(evidence.headers()["content-type"], "application/zip");
    assert_eq!(
        evidence.headers()["content-disposition"],
        "attachment; filename=\"acta.txt-evidence.zip\""
    );
    assert_eq!(evidence.headers()["x-document-digest"], "ab".repeat(32));
    assert_eq!(evidence.headers()["cache-control"], "no-store");
    assert_eq!(
        to_bytes(evidence.into_body(), 1024).await.unwrap(),
        "zip bytes"
    );
    let audit = router(&workflow)
        .oneshot(
            Request::get("/api/v1/audit/verify")
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(audit.status(), StatusCode::OK);
    assert_eq!(json(audit).await["entries"], 4);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn legacy_document_routes_are_absent_and_never_invoke_a_workflow() {
    let workflow = Arc::new(StubWorkflow::default());
    for token in ["owner-token", "client-token", "expired"] {
        for (method, path) in [
            ("POST", "/api/v1/documents".to_string()),
            ("POST", format!("/api/v1/documents/{DOCUMENT_UUID}/seal")),
            ("POST", format!("/api/v1/documents/{DOCUMENT_UUID}/verify")),
            ("GET", format!("/api/v1/documents/{DOCUMENT_UUID}/evidence")),
        ] {
            let response = router(&workflow)
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .header("authorization", format!("Bearer {token}"))
                        .header("x-document-name", "acta.txt")
                        .header("x-case-id", CASE_UUID.to_string())
                        .body(Body::from("case document"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn upload_requires_unambiguous_bearer_and_name_headers() {
    let workflow = Arc::new(StubWorkflow::default());
    for (headers, expected) in [
        (
            vec![("x-actor", "owner@example.com")],
            StatusCode::UNAUTHORIZED,
        ),
        (
            vec![("authorization", "Bearer owner-token")],
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            vec![
                ("authorization", "Bearer owner-token"),
                ("x-document-name", " "),
            ],
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            vec![
                ("authorization", "Bearer owner-token"),
                ("authorization", "Bearer owner-token"),
                ("x-document-name", "acta.txt"),
            ],
            StatusCode::UNAUTHORIZED,
        ),
        (
            vec![
                ("authorization", "Bearer owner-token"),
                ("x-document-name", "acta.txt"),
                ("x-document-name", "other.txt"),
            ],
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let mut request = Request::post(collection());
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = router(&workflow)
            .oneshot(request.body(Body::from("case document")).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn invalid_path_identifiers_never_reach_the_workflow() {
    let workflow = Arc::new(StubWorkflow::default());
    for (path, code) in [
        ("/api/v1/cases/invalid/documents".into(), "invalid_case_id"),
        (
            format!("{}/invalid/seal", collection()),
            "invalid_document_id",
        ),
        (
            format!("/api/v1/cases/invalid/documents/{DOCUMENT_UUID}/verify"),
            "invalid_case_id",
        ),
    ] {
        let response = router(&workflow)
            .oneshot(upload_request(&path))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json(response).await["error"]["code"], code);
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workflow_permission_errors_and_invalid_sessions_keep_their_status() {
    let workflow = Arc::new(StubWorkflow::default());
    for (token, expected) in [
        ("client-token", StatusCode::FORBIDDEN),
        ("expired", StatusCode::UNAUTHORIZED),
    ] {
        for (method, path) in [
            ("POST", collection()),
            ("POST", format!("{}/seal", document())),
            ("POST", format!("{}/verify", document())),
            ("GET", format!("{}/evidence", document())),
            ("GET", "/api/v1/audit/verify".into()),
        ] {
            let response = router(&workflow)
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .header("authorization", format!("Bearer {token}"))
                        .header("x-document-name", "acta.txt")
                        .body(Body::from("case document"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
        }
    }
}

#[tokio::test]
async fn foreign_and_unknown_documents_have_identical_not_found_responses() {
    let workflow = Arc::new(StubWorkflow::default());
    let mut errors = Vec::new();
    for path in [
        format!(
            "/api/v1/cases/{}/documents/{DOCUMENT_UUID}/verify",
            Uuid::from_u128(1)
        ),
        format!("{}/{}/verify", collection(), Uuid::from_u128(1)),
    ] {
        let response = router(&workflow)
            .oneshot(upload_request(&path))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        errors.push(json(response).await);
    }
    assert_eq!(errors[0], errors[1]);
    assert_eq!(errors[0]["error"]["code"], "document_not_found");
}

#[tokio::test]
async fn uploads_over_sixteen_mebibytes_are_rejected_before_the_workflow() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = router(&workflow)
        .oneshot(
            Request::post(collection())
                .header("authorization", "Bearer owner-token")
                .header("x-document-name", "acta.txt")
                .body(Body::from(vec![0; 16 * 1024 * 1024 + 1]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}
