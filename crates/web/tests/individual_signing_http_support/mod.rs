use crate::{document_signing_support as bindings, individual_document_support as docs};
use axum::{
    body::{to_bytes, Body},
    http::Request,
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

pub fn router(binding: bindings::Harness, document: docs::Harness) -> Router {
    web::individual_signing_router(
        web::IndividualSigningHttp {
            bindings: Arc::new(binding.service()),
            documents: Arc::new(document.service()),
        },
        web::HttpLimits::default(),
    )
    .unwrap()
}
pub fn binding_path(suffix: &str) -> String {
    format!(
        "/api/v1/identity/signing-certificates/{}/{suffix}",
        bindings::id()
    )
}
pub fn document_path(suffix: &str) -> String {
    format!(
        "/api/v1/cases/{}/documents/{}/versions/1/individual-signing/{suffix}",
        docs::case(),
        docs::reference().id
    )
}
pub fn submission() -> Value {
    json!({"consent_base64":STANDARD.encode(docs::consent().canonical_bytes()),
        "pdf_signature_base64":STANDARD.encode(docs::pdf_signature().as_bytes()),
        "consent_signature_base64":STANDARD.encode(docs::consent_signature().as_bytes())})
}
pub async fn request(app: Router, method: &str, path: &str, body: Value) -> (u16, Value) {
    let bytes = if body.is_null() {
        Vec::new()
    } else {
        serde_json::to_vec(&body).unwrap()
    };
    raw(
        app,
        method,
        path,
        bytes,
        &["Bearer session"],
        &["application/json"],
    )
    .await
}
pub async fn raw(
    app: Router,
    method: &str,
    path: &str,
    bytes: Vec<u8>,
    auth: &[&str],
    types: &[&str],
) -> (u16, Value) {
    let mut request = Request::builder().method(method).uri(path);
    for value in auth {
        request = request.header("authorization", *value);
    }
    for value in types {
        request = request.header("content-type", *value);
    }
    let request = request.body(Body::from(bytes)).unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(!response.headers().contains_key("authorization"));
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
