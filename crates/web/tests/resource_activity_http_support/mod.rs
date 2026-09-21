#![allow(dead_code)]
mod act;
#[path = "../deadline_http_support/mod.rs"]
mod deadlines;
mod model;
#[path = "../procedural_resource_http_support/model.rs"]
mod resources;
mod workflow;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
pub use model::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
pub use workflow::Workflow;
pub const CASE: &str = "00000000-0000-0000-0000-000000000001";
pub const RESOURCE: &str = "00000000-0000-0000-0000-000000000002";
pub const ID: &str = "00000000-0000-0000-0000-000000000003";
pub const OP: &str = "00000000-0000-0000-0000-000000000004";
pub const BEFORE: &str = "00000000-0000-0000-0000-000000000000";
pub const FOREIGN: &str = "00000000-0000-0000-0000-000000000099";
pub const HEARING: &str = "00000000-0000-0000-0000-000000000005";
pub const ACT: &str = "00000000-0000-0000-0000-000000000006";
pub fn base() -> String {
    format!("/api/v1/cases/{CASE}/procedural-resources/{RESOURCE}/activities")
}
pub fn command_json(action: &str, kind: &str) -> Value {
    let target = if kind == "hearing" {
        json!({"kind":kind,"id":HEARING,"revision":1,"submission_digest":"07".repeat(32)})
    } else {
        json!({"kind":kind,"id":BEFORE,"revision":1,"capture_digest":"07".repeat(32)})
    };
    let change = if action == "link" {
        json!({"action":action,"expected_revision":0,
            "resource":{"id":RESOURCE,"revision":2,"capture_digest":"07".repeat(32)},
            "act":null,"target":target})
    } else {
        json!({"action":action,"expected_revision":1,"reason":"Organizational unlink"})
    };
    json!({"case_id":CASE,"resource_id":RESOURCE,"association_id":ID,
        "operation_id":OP,"expected_resource_revision":5,"change":change})
}
pub fn submission(command: Value) -> Value {
    json!({"command":command,"expected_submission_digest":"07".repeat(32)})
}
pub async fn request(
    w: Arc<Workflow>,
    method: &str,
    path: &str,
    token: &str,
    body: Option<String>,
) -> (u16, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if !token.is_empty() {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    if body.is_some() {
        request = request.header("content-type", "application/json");
    }
    let response = web::resource_activity_router(w)
        .oneshot(
            request
                .body(body.map(Body::from).unwrap_or_else(Body::empty))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
