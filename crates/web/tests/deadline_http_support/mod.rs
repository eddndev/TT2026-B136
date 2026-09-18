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
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<String>,
    types: &[&str],
) -> (u16, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    for value in types {
        request = request.header("content-type", *value);
    }
    let response = web::deadline_router(workflow)
        .oneshot(
            request
                .body(body.map(Body::from).unwrap_or_else(Body::empty))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub async fn prepare(workflow: Arc<Workflow>, value: Value) -> (u16, Value) {
    request(
        workflow,
        "POST",
        &format!("{BASE}/prepare"),
        Some("owner"),
        Some(value.to_string()),
        &["application/json"],
    )
    .await
}
