#[allow(dead_code)]
mod hearing_result_support;
use hearing_result_support::*;
use serde_json::{json, Value};
use std::sync::Arc;

fn mutation(action: &str) -> (&'static str, String, Value) {
    let mut command = command_json();
    if action == "prepare" {
        return ("POST", format!("{}/prepare", base_url()), command);
    }
    let (method, suffix) = match action {
        "record" => ("POST", String::new()),
        "correct" => ("PUT", format!("/{RESULT}")),
        "withdraw" => ("POST", format!("/{RESULT}/withdrawal")),
        _ => panic!("unsupported test action"),
    };
    if action != "record" {
        command["change"] =
            json!({"action":action,"expected_revision":1,"reason":"Explicit reason"});
        if action == "correct" {
            command["change"]["values"] = values_json();
        }
    }
    (
        method,
        format!("{}{suffix}", base_url()),
        json!({"command":command,"expected_submission_digest":digest().to_hex()}),
    )
}
async fn rejects_queries(method: &str, path: &str, body: Option<Value>) {
    for query in [
        "unknown=x",
        "revision=1",
        "revision=1&revision=2",
        "invalid=%ZZ",
        "unexpected",
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, response) = request(
            workflow.clone(),
            method,
            &format!("{path}?{query}"),
            Some("valid"),
            body.as_ref().map(Value::to_string),
            if body.is_some() {
                &["application/json"]
            } else {
                &[]
            },
        )
        .await;
        assert_eq!(status, 400, "{method} {path}?{query}: {response}");
        assert_eq!(response["error"]["code"], "invalid_query");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn detail_rejects_queries_instead_of_ignoring_a_requested_revision() {
    rejects_queries("GET", &format!("{}/{RESULT}", base_url()), None).await;
}
#[tokio::test]
async fn exact_revision_rejects_unknown_and_duplicate_queries() {
    rejects_queries("GET", &format!("{}/{RESULT}/revisions/1", base_url()), None).await;
}
#[tokio::test]
async fn prepare_rejects_query_parameters() {
    let (method, path, body) = mutation("prepare");
    rejects_queries(method, &path, Some(body)).await;
}
#[tokio::test]
async fn record_rejects_query_parameters() {
    let (method, path, body) = mutation("record");
    rejects_queries(method, &path, Some(body)).await;
}
#[tokio::test]
async fn correct_rejects_query_parameters() {
    let (method, path, body) = mutation("correct");
    rejects_queries(method, &path, Some(body)).await;
}
#[tokio::test]
async fn withdraw_rejects_query_parameters() {
    let (method, path, body) = mutation("withdraw");
    rejects_queries(method, &path, Some(body)).await;
}
#[tokio::test]
async fn routes_without_query_options_accept_absent_or_empty_query() {
    for suffix in ["", "?"] {
        for path in [
            format!("{}/{RESULT}", base_url()),
            format!("{}/{RESULT}/revisions/1", base_url()),
        ] {
            let workflow = Arc::new(Workflow::default());
            let (status, response) = request(
                workflow.clone(),
                "GET",
                &format!("{path}{suffix}"),
                Some("valid"),
                None,
                &[],
            )
            .await;
            assert_eq!(status, 200, "{response}");
            assert_eq!(workflow.calls.lock().unwrap().len(), 1);
        }
        for action in ["prepare", "record", "correct", "withdraw"] {
            let (method, path, body) = mutation(action);
            let workflow = Arc::new(Workflow::default());
            let (status, response) = request(
                workflow.clone(),
                method,
                &format!("{path}{suffix}"),
                Some("valid"),
                Some(body.to_string()),
                &["application/json"],
            )
            .await;
            assert_eq!(
                status,
                if action == "prepare" { 200 } else { 201 },
                "{response}"
            );
            assert_eq!(workflow.calls.lock().unwrap().len(), 1);
        }
    }
}
