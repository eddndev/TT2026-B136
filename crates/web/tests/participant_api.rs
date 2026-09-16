mod participant_support;

use axum::{body::Body, http::StatusCode};
use participant_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn all_routes_forward_exact_scope_and_commands_without_followup_reads() {
    let workflow = Arc::new(Workflow::default());
    let response = request(
        &workflow,
        "POST",
        &base(),
        Some("owner"),
        json!({"display_name":"Ana","procedural_role":"Witness"}).to_string(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = request(
        &workflow,
        "PUT",
        &item(),
        Some("owner"),
        replacement(3).to_string(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = request(
        &workflow,
        "PUT",
        &format!("{}/directory-status", item()),
        Some("owner"),
        json!({"expected_revision":3,"directory_status":"archived"}).to_string(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = request(&workflow, "GET", &item(), Some("owner"), Body::empty()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![
            json!(["create", "owner", CASE, "Ana", "Witness", null, null]),
            json!(["replace","owner",CASE,ID,3,{
            "display_name":"Ana","procedural_role":"Witness","organization":null,
            "legal_status":null,"directory_status":"active"}]),
            json!(["status", "owner", CASE, ID, 3, "archived"]),
            json!(["get", "owner", CASE, ID]),
        ]
    );
}

#[tokio::test]
async fn snapshots_flatten_typed_values_and_preserve_captured_provenance_in_utc() {
    let workflow = Arc::new(Workflow::default());
    let response = request(&workflow, "GET", &item(), Some("owner"), Body::empty()).await;
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
        body(response).await,
        json!({
            "case_id":CASE,"id":ID,"revision":3,"display_name":"Ana",
            "procedural_role":"Witness","organization":null,"legal_status":null,
            "directory_status":"active","values_digest":"1a".repeat(32),
            "changed_at":"2023-11-14T22:13:20Z",
            "changed_by":{"id":ACTOR,"email":"historical@example.com"},
            "canonical_format":"part1","profile":null,"subject":null,
            "credential_origin":null,"submission_digest":null,"submission_revision":null,
        })
    );
}

#[tokio::test]
async fn list_uses_keyset_defaults_and_passes_literal_filters_to_the_workflow() {
    let workflow = Arc::new(Workflow::default());
    let response = request(&workflow, "GET", &base(), Some("reader"), Body::empty()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let result = body(response).await;
    assert_eq!(result["participants"][0]["id"], ID);
    assert_eq!(result["has_more"], true);
    assert_eq!(result["next_after_id"], ID);
    for field in [
        "values_digest",
        "legal_status",
        "changed_by",
        "profile",
        "credential_origin",
    ] {
        assert!(result["participants"][0].get(field).is_none(), "{field}");
    }
    let path = format!(
        "{}?limit=2&after_id={ID}&name=%20Ana%25_%20&procedural_role=%20Witness%20&status=all",
        base()
    );
    assert_eq!(
        request(&workflow, "GET", &path, Some("reader"), Body::empty())
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![
            json!(["list", "reader", CASE, 50, null, null, null, "active"]),
            json!(["list", "reader", CASE, 2, ID, "Ana%_", "Witness", null]),
        ]
    );
}

#[tokio::test]
async fn history_returns_original_snapshots_and_an_exclusive_revision_cursor() {
    let workflow = Arc::new(Workflow::default());
    let response = request(
        &workflow,
        "GET",
        &format!("{}/history?limit=1&before_revision=4", item()),
        Some("reader"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let result = body(response).await;
    assert_eq!(result["revisions"][0]["case_id"], CASE);
    assert_eq!(
        result["revisions"][0]["changed_by"]["email"],
        "historical@example.com"
    );
    assert_eq!(result["next_before_revision"], 3);
    assert_eq!(result["has_more"], true);
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![json!(["history", "reader", CASE, ID, 1, 4])]
    );
}

#[tokio::test]
async fn unauthenticated_requests_never_invoke_any_participant_workflow() {
    let workflow = Arc::new(Workflow::default());
    for (method, path, input) in [
        ("GET", base(), String::new()),
        ("GET", item(), String::new()),
        ("GET", format!("{}/history", item()), String::new()),
        (
            "POST",
            base(),
            json!({"display_name":"Ana","procedural_role":"Witness"}).to_string(),
        ),
        ("PUT", item(), replacement(3).to_string()),
        (
            "PUT",
            format!("{}/directory-status", item()),
            json!({"expected_revision":3,"directory_status":"active"}).to_string(),
        ),
    ] {
        let response = request(&workflow, method, &path, None, input).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body(response).await["error"]["code"], "invalid_session");
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn workflow_errors_keep_status_and_never_leak_stored_values_or_database_details() {
    let workflow = Arc::new(Workflow::default());
    for (token, status, code) in [
        ("expired", 401, "invalid_session"),
        ("forbidden", 403, "permission_denied"),
        ("hidden-case", 404, "case_not_found"),
        ("hidden", 404, "participant_not_found"),
        ("conflict", 409, "participant_revision_conflict"),
        ("exhausted", 409, "participant_revision_exhausted"),
        ("corrupt", 500, "internal_error"),
        ("failed", 500, "internal_error"),
    ] {
        let response = request(&workflow, "GET", &item(), Some(token), Body::empty()).await;
        assert_eq!(response.status().as_u16(), status, "{token}");
        let result = body(response).await;
        assert_eq!(result["error"]["code"], code);
        assert!(!result.to_string().contains("secret"));
    }
}

#[tokio::test]
async fn exact_revision_reads_forward_the_requested_version() {
    let workflow = Arc::new(Workflow::default());
    let response = request(
        &workflow,
        "GET",
        &format!("{}/revisions/2", item()),
        Some("reader"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![json!(["get_revision", "reader", CASE, ID, 2])]
    );
}

#[tokio::test]
async fn typed_kind_and_profile_filters_reach_authorized_listing() {
    let workflow = Arc::new(Workflow::default());
    let path = format!("{}?kind=control_judge&profile=typed&status=all", base());
    let response = request(&workflow, "GET", &path, Some("reader"), Body::empty()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        *workflow.filters.lock().unwrap(),
        vec![json!(["control_judge", "typed"])]
    );
    for suffix in [
        "?kind=judge",
        "?profile=complete",
        "?kind=control_judge&kind=expert",
    ] {
        assert_eq!(
            request(
                &workflow,
                "GET",
                &format!("{}{suffix}", base()),
                Some("reader"),
                Body::empty()
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(workflow.calls.lock().unwrap().len(), 1);
}
