use application::document_content::{DocumentContent, DocumentContentWorkflow};
use application::document_integrity::DocumentIntegrityFailure;
use application::ApplicationError;
use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use domain::{
    cases::CaseId,
    crypto::{DocumentVersion, DocumentVersionRef, Sha256Digest},
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tower::ServiceExt;
use uuid::Uuid;

struct Workflow {
    calls: AtomicUsize,
}

impl DocumentContentWorkflow for Workflow {
    fn content_version(
        &self,
        token: &str,
        case: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<DocumentContent, ApplicationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(token, "authorized-token");
        assert_eq!(reference.id.as_uuid(), Uuid::from_u128(2));
        assert_eq!(reference.version.get(), 1);
        if case.as_uuid() == Uuid::from_u128(9) {
            return Err(ApplicationError::PermissionDenied);
        }
        if case.as_uuid() == Uuid::from_u128(7) {
            return Err(ApplicationError::DocumentContentValidationFailed(
                DocumentIntegrityFailure::AuthenticationFailed,
            ));
        }
        if case.as_uuid() == Uuid::from_u128(6) {
            return Err(ApplicationError::DocumentContentTooLarge);
        }
        if case.as_uuid() == Uuid::from_u128(5) {
            return Err(ApplicationError::Port("private database diagnostic".into()));
        }
        Ok(DocumentContent {
            case_id: case,
            reference: if case.as_uuid() == Uuid::from_u128(8) {
                DocumentVersionRef {
                    id: reference.id,
                    version: DocumentVersion::new(2).unwrap(),
                }
            } else {
                reference
            },
            file_name: "legacy file.pdf".into(),
            digest: Sha256Digest::from_array([0xab; 32]),
            bytes: b"original\0content\xff".to_vec().into(),
        })
    }
}

fn path(case: u128, version: &str) -> String {
    format!(
        "/api/v1/cases/{}/documents/{}/versions/{version}/content",
        Uuid::from_u128(case),
        Uuid::from_u128(2)
    )
}

async fn request(
    workflow: &Arc<Workflow>,
    method: Method,
    path: &str,
    token: bool,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("range", "bytes=0-2");
    if token {
        request = request.header("authorization", "Bearer authorized-token");
    }
    web::document_content_router(workflow.clone())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn exact_content_is_binary_bound_to_case_and_version_without_range_or_cache_shortcuts() {
    let workflow = Arc::new(Workflow {
        calls: AtomicUsize::new(0),
    });
    let response = request(&workflow, Method::GET, &path(1, "1"), true).await;
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(headers["content-type"], "application/octet-stream");
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(headers["x-case-id"], Uuid::from_u128(1).to_string());
    assert_eq!(headers["x-document-id"], Uuid::from_u128(2).to_string());
    assert_eq!(headers["x-document-version"], "1");
    assert_eq!(headers["x-document-digest"], "ab".repeat(32));
    assert!(headers["content-disposition"]
        .to_str()
        .unwrap()
        .starts_with("attachment;"));
    assert!(!headers.contains_key("etag"));
    assert!(!headers.contains_key("content-range"));
    assert_eq!(
        &to_bytes(response.into_body(), 1024).await.unwrap()[..],
        b"original\0content\xff"
    );
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn missing_bearer_head_and_invalid_exact_queries_do_not_authorize_a_content_read() {
    let workflow = Arc::new(Workflow {
        calls: AtomicUsize::new(0),
    });
    let cases = [
        (Method::GET, path(1, "1"), false, StatusCode::UNAUTHORIZED),
        (
            Method::HEAD,
            path(1, "1"),
            true,
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (Method::GET, path(1, "0"), true, StatusCode::BAD_REQUEST),
        (Method::GET, path(1, "01"), true, StatusCode::BAD_REQUEST),
        (
            Method::GET,
            format!("{}?current=true", path(1, "1")),
            true,
            StatusCode::BAD_REQUEST,
        ),
    ];
    for (method, path, token, expected) in cases {
        let response = request(&workflow, method, &path, token).await;
        assert_eq!(response.status(), expected, "{path}");
        assert!(!response.headers().contains_key("content-disposition"));
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn denied_and_mismatched_workflow_results_never_release_content() {
    let workflow = Arc::new(Workflow {
        calls: AtomicUsize::new(0),
    });
    for (case, expected) in [
        (9, StatusCode::FORBIDDEN),
        (8, StatusCode::INTERNAL_SERVER_ERROR),
    ] {
        let response = request(&workflow, Method::GET, &path(case, "1"), true).await;
        assert_eq!(response.status(), expected);
        assert!(!response.headers().contains_key("content-disposition"));
        assert!(!response.headers().contains_key("x-document-digest"));
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"]
                .is_string()
        );
        assert!(!body.windows(8).any(|part| part == b"original"));
    }
}

#[tokio::test]
async fn failed_validation_and_unavailability_are_distinct_and_never_downloadable() {
    let workflow = Arc::new(Workflow {
        calls: AtomicUsize::new(0),
    });
    for (case, status, code) in [
        (
            7,
            StatusCode::CONFLICT,
            "document_content_validation_failed",
        ),
        (
            6,
            StatusCode::PAYLOAD_TOO_LARGE,
            "document_content_too_large",
        ),
        (5, StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
    ] {
        let response = request(&workflow, Method::GET, &path(case, "1"), true).await;
        assert_eq!(response.status(), status);
        assert!(!response.headers().contains_key("content-disposition"));
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"]["code"], code);
        assert!(!String::from_utf8_lossy(&bytes).contains("private database"));
        assert!(!String::from_utf8_lossy(&bytes).contains("AuthenticationFailed"));
    }
}
