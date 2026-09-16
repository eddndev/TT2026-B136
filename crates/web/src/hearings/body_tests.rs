use super::route_support::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn raw(body: String, content_types: &[&str]) -> (u16, Value, usize) {
    let w = Arc::new(Workflow::default());
    let mut request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/cases/{CASE}/hearings/prepare"))
        .header("authorization", "Bearer valid");
    for value in content_types {
        request = request.header("content-type", *value);
    }
    let response = crate::hearing_router(w.clone())
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let calls = w.calls.lock().unwrap().len();
    (status, serde_json::from_slice(&bytes).unwrap(), calls)
}
#[tokio::test]
async fn body_requires_one_json_content_type_and_no_suffix() {
    let text = super::value_tests::command().to_string();
    for headers in [
        vec![],
        vec!["text/plain"],
        vec!["application/json", "application/json"],
    ] {
        let (status, _, calls) = raw(text.clone(), &headers).await;
        assert_eq!(status, 400);
        assert_eq!(calls, 0);
    }
    for suffix in ["{}", "null", " trailing"] {
        let (status, _, calls) = raw(format!("{text}{suffix}"), &["application/json"]).await;
        assert_eq!(status, 400);
        assert_eq!(calls, 0);
    }
}
#[tokio::test]
async fn entity_size_is_bounded_before_parsing_or_workflow() {
    let (status, body, calls) = raw(" ".repeat(64 * 1024 + 1), &["application/json"]).await;
    assert_eq!(status, 413);
    assert_eq!(body["error"]["code"], "hearing_body_too_large");
    assert_eq!(calls, 0);
}
#[tokio::test]
async fn object_contract_rejects_positional_arrays_at_every_object_boundary() {
    let c = super::value_tests::command();
    let positional = json!([c["operation_id"], c["hearing_id"], c["change"]]);
    let mut nested = c.clone();
    let v = &c["change"]["values"];
    nested["change"]["values"] = json!([
        v["kind"],
        v["scheduled_at"],
        v["modality"],
        v["venue"],
        null,
        [],
        null
    ]);
    let mut participant = c.clone();
    participant["change"]["values"]["participants"] =
        json!([["11111111-1111-4111-8111-111111111111", 1]]);
    for value in [positional, nested, participant] {
        let (status, _, calls) = raw(value.to_string(), &["application/json"]).await;
        assert_eq!(status, 400, "accepted positional JSON");
        assert_eq!(calls, 0);
    }
}
#[tokio::test]
async fn duplicate_nested_fields_are_rejected_before_workflow() {
    let text = super::value_tests::command()
        .to_string()
        .replace("\"venue\":", "\"venue\":\"first\",\"venue\":");
    let (status, _, calls) = raw(text, &["application/json"]).await;
    assert_eq!(status, 400);
    assert_eq!(calls, 0);
}
