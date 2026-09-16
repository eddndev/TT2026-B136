#![allow(dead_code)]
mod model;
#[path = "../../../domain/tests/procedural_fact_support/fixtures.rs"]
pub mod value_factory;
mod workflow;
use application::procedural_facts::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId};
pub use model::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;
pub use workflow::Workflow;

pub const CASE: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
pub const PARENT: &str = "00000000-0000-4000-8000-000000000010";
pub const ID: &str = "00000000-0000-4000-8000-000000000020";
pub const OPERATION: &str = "00000000-0000-4000-8000-000000000030";
pub const NIL: &str = "00000000-0000-0000-0000-000000000000";
pub fn case() -> CaseId {
    CaseId::from_uuid(Uuid::parse_str(CASE).unwrap())
}
pub fn actor() -> UserId {
    UserId::from_uuid(Uuid::from_u128(99))
}
pub fn digest() -> Sha256Digest {
    Sha256Digest::from_array([0x66; 32])
}
pub fn text(v: &str) -> FactText {
    FactText::new(v).unwrap()
}
pub fn label(v: &str) -> FactLabel {
    FactLabel::new(v).unwrap()
}
pub fn vectors() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../domain/tests/fixtures/procedural_fact_vectors.json"
    ))
    .unwrap()
}
pub fn values_json(family: &str) -> Value {
    let mut v = vectors()
        .into_iter()
        .find(|v| v["name"] == format!("{family}_minimum"))
        .unwrap()["normalized"]
        .clone();
    if family == "notification" {
        v["resolution"]["id"] = json!(PARENT);
    }
    v
}
pub fn command_json(family: &str, action: &str) -> Value {
    let mut command = json!({"family":family,"operation_id":OPERATION,"id":ID,
        "change":{"action":action,"expected_revision":if action=="record" {0} else {1}}});
    if family == "notification" {
        command["resolution_id"] = json!(PARENT);
    }
    if action != "withdraw" {
        command["change"]["values"] = values_json(family);
    }
    if action != "record" {
        command["change"]["reason"] = json!("Explicit reason");
    }
    command
}
pub fn submission(command: Value) -> Value {
    json!({"command":command,"expected_submission_digest":digest().to_hex()})
}
pub fn base_url(family: &str) -> String {
    let root = format!("/api/v1/cases/{CASE}/resolutions");
    if family == "notification" {
        format!("{root}/{PARENT}/notifications")
    } else {
        root
    }
}
pub fn target_json(target: FactTarget) -> Value {
    match target {
        FactTarget::Resolution(id) => json!({"family":"resolution","id":id.to_string()}),
        FactTarget::Notification { id, resolution_id } => {
            json!({"family":"notification","id":id.to_string(),"resolution_id":resolution_id.to_string()})
        }
    }
}
pub fn action_name(action: FactAction) -> &'static str {
    match action {
        FactAction::Record => "record",
        FactAction::Correct => "correct",
        FactAction::Withdraw => "withdraw",
    }
}
pub fn status_name(status: FactStatus) -> &'static str {
    match status {
        FactStatus::Recorded => "recorded",
        FactStatus::Withdrawn => "withdrawn",
    }
}
pub fn typed_command(v: &Value) -> ProceduralFactCommand {
    fn change<T>(v: &Value, make: impl FnOnce(&Value) -> T) -> FactChange<T> {
        match v["action"].as_str().unwrap() {
            "record" => FactChange::record(make(&v["values"])),
            "correct" => FactChange::correct(
                FactRevision::new(v["expected_revision"].as_u64().unwrap() as u32).unwrap(),
                make(&v["values"]),
                text(v["reason"].as_str().unwrap()),
            ),
            "withdraw" => FactChange::withdraw(
                FactRevision::new(v["expected_revision"].as_u64().unwrap() as u32).unwrap(),
                text(v["reason"].as_str().unwrap()),
            ),
            _ => panic!("fixture action"),
        }
    }
    let op =
        FactOperationId::from_uuid(Uuid::parse_str(v["operation_id"].as_str().unwrap()).unwrap());
    let id = Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();
    if v["family"] == "resolution" {
        ProceduralFactCommand::Resolution(ResolutionCommand::new(
            op,
            ResolutionId::from_uuid(id),
            change(&v["change"], value_factory::resolution),
        ))
    } else {
        ProceduralFactCommand::Notification(
            NotificationCommand::new(
                op,
                NotificationId::from_uuid(id),
                ResolutionId::from_uuid(
                    Uuid::parse_str(v["resolution_id"].as_str().unwrap()).unwrap(),
                ),
                change(&v["change"], value_factory::notification),
            )
            .unwrap(),
        )
    }
}
pub async fn request(
    workflow: Arc<Workflow>,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<String>,
    types: &[&str],
) -> (u16, Value) {
    let mut headers = Vec::new();
    if let Some(token) = token {
        headers.push(("authorization", format!("Bearer {token}")));
    }
    headers.extend(types.iter().map(|v| ("content-type", (*v).to_owned())));
    request_headers(workflow, method, path, body, &headers).await
}
pub async fn request_headers(
    workflow: Arc<Workflow>,
    method: &str,
    path: &str,
    body: Option<String>,
    headers: &[(&str, String)],
) -> (u16, Value) {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    let response = web::procedural_fact_router(workflow)
        .oneshot(
            request
                .body(body.map(Body::from).unwrap_or_else(Body::empty))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({"raw":String::from_utf8_lossy(&bytes)})),
    )
}
