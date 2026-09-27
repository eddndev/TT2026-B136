#[path = "deadline_http_support/values.rs"]
#[allow(dead_code)]
mod deadlines;
use application::{resource_deadlines::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{cases::CaseId, crypto::Sha256Digest, procedural_resources::ResourceId};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
const RESOURCE: &str = "00000000-0000-0000-0000-000000000012";
const ASSOCIATION: &str = "00000000-0000-0000-0000-000000000013";
#[derive(Default)]
struct Workflow {
    calls: Mutex<Vec<(String, ResourceDeadlineCommand)>>,
}
impl ResourceDeadlineWorkflow for Workflow {
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceDeadlineCommand,
    ) -> Result<ResourceDeadlineDraft, ApplicationError> {
        assert_eq!(case, deadlines::case());
        assert_eq!(resource.to_string(), RESOURCE);
        self.calls.lock().unwrap().push((token.into(), command));
        Err(ApplicationError::PermissionDenied)
    }
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceDeadlineCommand,
        expected: Sha256Digest,
    ) -> Result<ResourceDeadlineResult, ApplicationError> {
        assert_eq!(case, deadlines::case());
        assert_eq!(resource.to_string(), RESOURCE);
        assert_eq!(expected, deadlines::digest());
        self.calls.lock().unwrap().push((token.into(), command));
        Err(ApplicationError::PermissionDenied)
    }
}
fn command() -> Value {
    json!({"case_id":deadlines::CASE,"resource_id":RESOURCE,"association_id":ASSOCIATION,
        "expected_resource_revision":2,"resource":{"id":RESOURCE,"revision":1,"capture_digest":"07".repeat(32)},
        "act":null,"deadline":deadlines::command()})
}
async fn request(
    w: Arc<Workflow>,
    suffix: &str,
    token: &str,
    body: String,
    content_type: &str,
) -> (u16, Value) {
    let path = format!(
        "/api/v1/cases/{}/procedural-resources/{RESOURCE}/activities/deadlines/{suffix}",
        deadlines::CASE
    );
    let mut request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", content_type);
    if !token.is_empty() {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = web::resource_deadline_router(w)
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn contextual_commands_reach_one_workflow_without_an_ordinary_deadline_write() {
    for suffix in ["prepare", "submit"] {
        let w = Arc::new(Workflow::default());
        let value = if suffix == "prepare" {
            command()
        } else {
            json!({"command":command(),"expected_submission_digest":deadlines::digest().to_hex()})
        };
        let (status, body) = request(
            w.clone(),
            suffix,
            "owner",
            value.to_string(),
            "application/json",
        )
        .await;
        assert_eq!(status, 403, "{body}");
        let calls = w.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "owner");
        assert_eq!(calls[0].1.association_id.to_string(), ASSOCIATION);
        assert_eq!(calls[0].1.expected_resource_revision.get(), 2);
        assert_eq!(calls[0].1.resource.revision.get(), 1);
        assert!(calls[0].1.act.is_none());
        let (deadline, policies) = calls[0].1.deadline.clone().into_parts();
        assert_eq!(deadline.operation_id.to_string(), deadlines::ID);
        assert!(policies.is_some());
    }
}
#[tokio::test]
async fn contextual_input_rejects_foreign_parents_wrong_actions_and_unknown_fields() {
    let mut invalid = Vec::new();
    for field in ["case_id", "resource_id"] {
        let mut v = command();
        v[field] = json!(ASSOCIATION);
        invalid.push(v);
    }
    let mut v = command();
    v["resource"]["id"] = json!(ASSOCIATION);
    invalid.push(v);
    let mut v = command();
    v["deadline"]["change"]["definition"]["input"]["selection"]["case_id"] = json!(ASSOCIATION);
    invalid.push(v);
    let mut v = command();
    v["deadline"]["change"] = json!({"action":"retire","expected_revision":1,"reason":"retire"});
    invalid.push(v);
    let mut v = command();
    v["invented"] = json!(true);
    invalid.push(v);
    let mut v = command();
    v.as_object_mut().unwrap().remove("act");
    invalid.push(v);
    for value in invalid {
        let w = Arc::new(Workflow::default());
        let (status, body) = request(
            w.clone(),
            "prepare",
            "owner",
            value.to_string(),
            "application/json",
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(w.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn contextual_transport_enforces_authentication_query_and_json_budget() {
    let encoded = command().to_string();
    let duplicate = encoded.replacen("{", "{\"case_id\":\"duplicate\",", 1);
    for (suffix, token, body, content_type, expected) in [
        ("prepare", "", encoded.clone(), "application/json", 401),
        (
            "prepare?filter=any",
            "owner",
            encoded.clone(),
            "application/json",
            400,
        ),
        ("prepare", "owner", encoded.clone(), "text/plain", 400),
        ("prepare", "owner", duplicate, "application/json", 400),
        (
            "prepare",
            "owner",
            " ".repeat(1024 * 1024 + 1),
            "application/json",
            413,
        ),
    ] {
        let w = Arc::new(Workflow::default());
        let (status, value) = request(w.clone(), suffix, token, body, content_type).await;
        assert_eq!(status, expected, "{value}");
        assert!(w.calls.lock().unwrap().is_empty());
    }
}
