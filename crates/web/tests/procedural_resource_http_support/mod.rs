#![allow(dead_code)]
mod model;
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
pub const ID: &str = "00000000-0000-0000-0000-000000000002";
pub const OP: &str = "00000000-0000-0000-0000-000000000003";
pub const ACT: &str = "00000000-0000-0000-0000-000000000004";
pub fn base() -> String {
    format!("/api/v1/cases/{CASE}/procedural-resources")
}
pub fn evidence() -> Value {
    json!({"document_id":"00000000-0000-0000-0000-000000000014","version":3,
        "digest":"2a".repeat(32),"locator":"Order page 2"})
}
pub fn values_json() -> Value {
    json!({"kind":"revocation","mode":{"kind":"known","value":"written"},
        "title":"Written revocation record","resolution":{"id":"00000000-0000-0000-0000-00000000000a","revision":2},
        "resolution_evidence":evidence(),"resolution_reference":{"kind":"known","value":"Order reference"},
        "issuing_authority":{"kind":"unknown","reason":"Issuer not recorded"},
        "receiving_authority":null,"resolution_at":{"precision":"unknown"},"notification_at":null,
        "challenged_part":"Declared challenged paragraph","grounds":"Declared reasons",
        "appellants":[{"name":"Captured name","role":{"kind":"known","value":"Affected party"},"participant":null}]})
}
pub fn act_json() -> Value {
    json!({"kind":"interposition","mode":{"kind":"known","value":"oral"},
        "occurred_at":{"precision":"date","year":2026,"month":9,"day":19,"offset_seconds":null},
        "authority":{"kind":"unknown","reason":"Not recorded"},"statement":"Declared oral act",
        "evidence":[evidence()]})
}
pub fn command_json(action: &str) -> Value {
    let mut change =
        json!({"action":action,"expected_revision":if action=="register" {0} else {1}});
    if matches!(action, "register" | "correct") {
        change["values"] = values_json();
    }
    if matches!(action, "record_act" | "correct_act") {
        change["act_id"] = json!(ACT);
        change["values"] = act_json();
    }
    if action == "correct_act" {
        change["expected_act_revision"] = json!(1);
    }
    if matches!(action, "correct" | "correct_act" | "archive" | "reactivate") {
        change["reason"] = json!("Explicit correction or organizational reason");
    }
    json!({"operation_id":OP,"resource_id":ID,"change":change})
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
    let response = web::procedural_resource_router(w)
        .oneshot(
            request
                .body(body.map(Body::from).unwrap_or_else(Body::empty))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
