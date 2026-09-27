mod document_support;

use std::sync::{atomic::Ordering, Arc};

use application::documents::DocumentUploadError;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use document_support::{StubIdentity, StubWorkflow, CASE_UUID, DOCUMENT_UUID};
use serde_json::json;
use tower::ServiceExt;

fn upload_request(ingress: &str) -> Request<Body> {
    let collection = format!("/api/v1/cases/{CASE_UUID}/documents");
    let (path, name, content_type, content) = match ingress {
        "plain" => (collection, "acta.txt", "application/octet-stream", "case document".to_string()),
        "append" => (
            format!("{collection}/{DOCUMENT_UUID}/versions?expected_version=3"),
            "revised.txt", "application/octet-stream", "revised content".to_string(),
        ),
        "classified" => (
            format!("{collection}/with-metadata"), "acta.txt", "multipart/form-data; boundary=admission",
            concat!("--admission\r\nContent-Disposition: form-data; name=\"file\"; filename=\"ignored.bin\"\r\n\r\ncase document\r\n",
                "--admission\r\nContent-Disposition: form-data; name=\"metadata\"\r\n\r\n",
                "{\"document_type\":\"Escrito\",\"classification\":null,\"tags\":[]}\r\n--admission--\r\n").to_string(),
        ),
        _ => unreachable!(),
    };
    Request::post(path)
        .header("authorization", "Bearer owner-token")
        .header("x-document-name", name)
        .header("content-type", content_type)
        .body(Body::from(content))
        .unwrap()
}

#[tokio::test]
async fn all_upload_routes_preserve_admission_errors_and_prevent_caching() {
    for (cause, status, code, message) in [
        (
            DocumentUploadError::Unsupported,
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_format_unsupported",
            "document format is not supported",
        ),
        (
            DocumentUploadError::Invalid,
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_format_invalid",
            "document format is invalid",
        ),
        (
            DocumentUploadError::Limit,
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_validation_limit",
            "document validation exceeded its resource limit",
        ),
        (
            DocumentUploadError::Unavailable,
            StatusCode::SERVICE_UNAVAILABLE,
            "document_validator_unavailable",
            "document validator is unavailable",
        ),
    ] {
        let workflow = Arc::new(StubWorkflow {
            upload_failure: Some(cause),
            ..Default::default()
        });
        let router = web::application_router(workflow.clone(), Arc::new(StubIdentity));
        for ingress in ["plain", "classified", "append"] {
            let response = router
                .clone()
                .oneshot(upload_request(ingress))
                .await
                .unwrap();
            assert_eq!(response.status(), status, "{ingress} {cause:?}");
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(response.headers()["content-type"], "application/json");
            let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body, json!({"error":{"code":code,"message":message}}));
        }
        assert_eq!(workflow.calls.load(Ordering::SeqCst), 3);
    }
}
