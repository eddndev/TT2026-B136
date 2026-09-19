mod alert_http_support;
use alert_http_support::*;
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

#[tokio::test]
async fn preferences_expose_personal_defaults_and_bind_committed_commands() {
    let (_, router) = setup(false);
    let (status, body) = request(
        router.clone(),
        "GET",
        "/api/v1/alert-preferences",
        None,
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["preferences"]["user_id"], actor().to_string());
    assert_eq!(body["preferences"]["revision"], 0);
    assert_eq!(body["preferences"]["receipt"], Value::Null);
    assert_eq!(body["preferences"]["values"], preference_body()["values"]);
    let submitted = preference_body();
    let (status, body) = request(
        router,
        "PUT",
        "/api/v1/alert-preferences",
        Some(&submitted.to_string()),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["preferences"]["revision"], 1);
    assert_eq!(body["preferences"]["values"], submitted["values"]);
    assert_eq!(
        body["preferences"]["receipt"]["operation_id"],
        submitted["operation_id"]
    );
}

#[tokio::test]
async fn inbox_keeps_exact_captured_context_and_provider_acceptance_separate_from_reading() {
    let (_, router) = setup(false);
    let (status, body) = request(
        router.clone(),
        "GET",
        "/api/v1/alerts?limit=20&read=all&state=active",
        None,
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["alerts"][0]["case_reference"], "CASE-4");
    assert_eq!(body["alerts"][0]["subject_title"], "Captured deadline");
    assert_eq!(body["alerts"][0]["origin"]["revision"], 2);
    assert_eq!(body["alerts"][0]["email"]["kind"], "accepted");
    assert_eq!(body["alerts"][0]["read_at"], Value::Null);
    assert_eq!(body["checked_at"]["nanosecond"], 123456789);
    assert!(!body.to_string().contains("recipient_email"));
    assert!(!body.to_string().contains("provider_id"));
    let submitted = json!({"operation_id":uuid::Uuid::from_u128(9).to_string()});
    let (status, read) = request(
        router,
        "POST",
        &format!("/api/v1/alerts/{}/read", id()),
        Some(&submitted.to_string()),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read["operation_id"], submitted["operation_id"]);
    assert_ne!(read["alert"]["read_at"], Value::Null);
    assert_eq!(read["alert"]["subject"], body["alerts"][0]["subject"]);
    assert_eq!(read["alert"]["state"], json!({"kind":"active"}));
}

#[tokio::test]
async fn authentication_and_permission_failures_do_not_disclose_preferences_or_inbox() {
    for path in ["/api/v1/alerts", "/api/v1/alert-preferences"] {
        let (workflow, router) = setup(false);
        assert_eq!(
            request(router, "GET", path, None, false).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
        let (_, router) = setup(true);
        assert_eq!(
            request(router, "GET", path, None, true).await.0,
            StatusCode::FORBIDDEN
        );
    }
}

#[tokio::test]
async fn malformed_or_ambiguous_queries_and_foreign_target_fields_never_reach_workflow() {
    for query in [
        "limit=0",
        "limit=101",
        "read=read",
        "state=hidden",
        "limit=1&limit=2",
        "recipient_id=other",
        "cursor=bad",
    ] {
        let (workflow, router) = setup(false);
        let (status, _) = request(
            router,
            "GET",
            &format!("/api/v1/alerts?{query}"),
            None,
            true,
        )
        .await;
        assert!(status.is_client_error(), "{query}");
        assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
    }
    for field in ["recipient_id", "user_id", "email"] {
        let (workflow, router) = setup(false);
        let mut body = preference_body();
        body[field] = json!("untrusted");
        assert_eq!(
            request(
                router,
                "PUT",
                "/api/v1/alert-preferences",
                Some(&body.to_string()),
                true
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn duplicate_nested_fields_invalid_leads_and_trailing_json_are_rejected() {
    let valid = preference_body().to_string();
    for body in [
        valid.replace("\"internal\":true", "\"internal\":true,\"internal\":false"),
        valid.replace("[48,24]", "[48,48]"),
        format!("{valid} {{}}"),
    ] {
        let (workflow, router) = setup(false);
        assert!(request(
            router,
            "PUT",
            "/api/v1/alert-preferences",
            Some(&body),
            true
        )
        .await
        .0
        .is_client_error());
        assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn absent_or_inaccessible_alerts_share_one_error_and_stale_preferences_conflict() {
    let (_, router) = setup(false);
    let (status, error) = request(
        router.clone(),
        "GET",
        "/api/v1/alerts/00000000-0000-0000-0000-000000000099",
        None,
        true,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "alert_not_found");
    let mut body = preference_body();
    body["expected_revision"] = json!(1);
    let (status, error) = request(
        router,
        "PUT",
        "/api/v1/alert-preferences",
        Some(&body.to_string()),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "alert_revision_conflict");
}
