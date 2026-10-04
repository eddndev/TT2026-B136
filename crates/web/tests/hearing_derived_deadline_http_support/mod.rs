#![allow(dead_code)]
pub mod records;
mod values;
mod workflow;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;
pub use values::*;
pub use workflow::*;

pub async fn request(
    workflow: Arc<Workflow>,
    suffix: &str,
    token: Option<&str>,
    body: String,
    content_types: &[&str],
) -> (u16, Value) {
    let mut request = Request::builder()
        .method("POST")
        .uri(format!("{}{suffix}", base()));
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    for value in content_types {
        request = request.header("content-type", *value);
    }
    let response = web::hearing_derived_deadline_router(workflow)
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

pub async fn prepare(workflow: Arc<Workflow>, value: Value) -> (u16, Value) {
    request(
        workflow,
        "/prepare",
        Some("owner"),
        value.to_string(),
        &["application/json"],
    )
    .await
}
