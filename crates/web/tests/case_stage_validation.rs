mod case_stage_support;
use axum::http::StatusCode;
use case_stage_support::*;
use serde_json::{json, Value};
use std::sync::Arc;

async fn rejected(suffix: &str, value: Value, code: &str) {
    let workflow = Arc::new(Workflow::default());
    let response = call(workflow.clone(), "POST", suffix, "owner", value).await;
    assert!(
        response.status().is_client_error(),
        "expected rejection for {code}"
    );
    assert_eq!(body(response).await["error"]["code"], code);
    assert!(workflow.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn strict_variants_reject_unknown_and_foreign_fields() {
    for (suffix, mut value, field) in [
        ("/adoption", adoption(), "target"),
        ("/transitions", intermediate(), "received_at"),
        ("/transitions", trial(), "accusation"),
    ] {
        value[field] = json!("unexpected");
        rejected(suffix, value, "invalid_json").await;
    }
    let mut value = adoption();
    value["support"]["name"] = json!("untrusted.pdf");
    rejected("/adoption", value, "invalid_json").await;
    let mut value = adoption();
    value["known_at"]["at"] = json!("2023-01-01T10:00:00Z");
    rejected("/adoption", value, "invalid_json").await;
    let mut value = intermediate();
    value["accusation_declared_at"]["offset"] = json!("+00:00");
    rejected("/transitions", value, "invalid_json").await;
}
#[tokio::test]
async fn adoption_zero_and_transition_positive_expectations_are_enforced() {
    let mut value = adoption();
    value["expected_revision"] = json!(1);
    rejected("/adoption", value, "invalid_case_stage_revision").await;
    let mut value = intermediate();
    value["expected_revision"] = json!(0);
    rejected("/transitions", value, "invalid_case_stage_revision").await;
    let mut value = adoption();
    value["stage"] = json!("appeal");
    rejected("/adoption", value, "invalid_case_stage").await;
    let mut value = intermediate();
    value["target"] = json!("investigation");
    rejected("/transitions", value, "invalid_json").await;
}
#[tokio::test]
async fn declared_dates_require_valid_calendar_and_canonical_explicit_offsets() {
    for (date, offset) in [
        ("2023-02-29", "-06:00"),
        ("0000-01-01", "+00:00"),
        ("2023-1-02", "+00:00"),
        ("2023-01-02", "Z"),
        ("2023-01-02", "-00:00"),
        ("2023-01-02", "+14:01"),
        ("2023-01-02", "+01:60"),
        ("2023-01-02", "+6:00"),
        ("2023-01-02", "+06:00:00"),
    ] {
        let mut value = adoption();
        value["known_at"] = json!({"precision":"date","date":date,"offset":offset});
        rejected("/adoption", value, "invalid_declared_stage_time").await;
    }
    for at in [
        "2023-01-01T00:00:00-00:00",
        "2023-01-01T00:00:00+14:01",
        "2023-01-01T00:00:00",
        "2023-02-29T00:00:00Z",
        "2023-01-01T00:00:60Z",
        "2023-01-01T00:00:00.1234567891Z",
        "bad\u{1f512}",
    ] {
        let mut value = adoption();
        value["known_at"] = json!({"precision":"instant","at":at});
        rejected("/adoption", value, "invalid_declared_stage_time").await;
    }
}
#[tokio::test]
async fn date_and_instant_boundary_offsets_are_accepted_without_losing_precision() {
    for known in [
        json!({"precision":"date","date":"2024-02-29","offset":"+14:00"}),
        json!({"precision":"date","date":"2024-02-29","offset":"-14:00"}),
        json!({"precision":"instant","at":"2024-02-29T12:00:00Z"}),
    ] {
        let mut value = adoption();
        value["known_at"] = known.clone();
        let response = call(
            Arc::new(Workflow::default()),
            "POST",
            "/adoption",
            "owner",
            value,
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(body(response).await["current"]["values"]["known_at"], known);
    }
}
#[tokio::test]
async fn support_identity_version_and_digest_are_required_and_validated() {
    for (key, value, code) in [
        ("document_id", json!("bad"), "invalid_document_id"),
        ("version", json!(0), "invalid_document_version"),
        (
            "digest",
            json!("not a digest"),
            "invalid_stage_support_digest",
        ),
    ] {
        let mut input = adoption();
        input["support"][key] = value;
        rejected("/adoption", input, code).await;
    }
}
#[tokio::test]
async fn domain_text_order_and_conflicting_support_limits_are_applied() {
    for reason in [
        " ".into(),
        "a".repeat(1001),
        "valid\u{0009}".into(),
        "valid\u{0085}".into(),
    ] {
        let mut value = adoption();
        value["reason"] = json!(reason);
        rejected("/adoption", value, "invalid_stage_note").await;
    }
    for (key, text, code) in [
        ("receiving_court", "a".repeat(201), "invalid_stage_court"),
        (
            "receiving_court",
            "court\nnew".into(),
            "invalid_stage_court",
        ),
        (
            "receipt_reference",
            "a".repeat(201),
            "invalid_stage_receipt_reference",
        ),
        ("note", "a".repeat(1001), "invalid_stage_note"),
    ] {
        let mut value = trial();
        value[key] = json!(text);
        rejected("/transitions", value, code).await;
    }
    let mut value = trial();
    value["received_at"]["date"] = json!("2022-01-02");
    rejected("/transitions", value, "invalid_stage_act_order").await;
    let mut value = trial();
    value["receipt_support"]["digest"] = json!("cd".repeat(32));
    rejected("/transitions", value, "conflicting_stage_support").await;
}
#[tokio::test]
async fn pagination_rejects_unknown_nonpositive_and_overflow_values() {
    for query in [
        "limit=0",
        "limit=101",
        "limit=-1",
        "limit=4294967296",
        "before_revision=0",
        "before_revision=-1",
        "before_revision=x",
        "offset=1",
        "limit=1&limit=2",
    ] {
        let workflow = Arc::new(Workflow::default());
        let response = call(
            workflow.clone(),
            "GET",
            &format!("/history?{query}"),
            "owner",
            Value::Null,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "query {query}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn bearer_syntax_precedes_body_validation_and_body_has_a_strict_budget() {
    let workflow = Arc::new(Workflow::default());
    let response = raw(workflow.clone(), "POST", "/adoption", None, "{".into()).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = raw(
        workflow.clone(),
        "POST",
        "/adoption",
        Some("owner"),
        "{".into(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let mut value = adoption();
    value["reason"] = json!("a".repeat(32 * 1024));
    let response = call(workflow.clone(), "POST", "/adoption", "owner", value).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        body(response).await["error"]["code"],
        "case_stage_body_too_large"
    );
    assert!(workflow.calls.lock().unwrap().is_empty());
}
