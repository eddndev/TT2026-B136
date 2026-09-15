mod case_administration_support;

use axum::{body::Body, http::StatusCode};
use case_administration_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn penal_creation_normalizes_values_and_returns_its_committed_projection() {
    let workflow = Arc::new(Workflow::default());
    let response = request(
        &workflow,
        "POST",
        "/api/v1/penal-cases",
        Some("owner"),
        creation().to_string(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let result = body(response).await;
    assert_eq!(result["id"], CASE);
    assert_eq!(result["administration"]["case_id"], CASE);
    assert_eq!(result["administration"]["revision"], 3);
    assert_eq!(
        result["administration"]["changed_by"]["email"],
        "captured@example.com"
    );
    assert_eq!(
        result["administration"]["changed_at"],
        "2023-11-14T22:13:20Z"
    );
    assert_eq!(
        result["administration"]["profile"]["general_information"],
        "Line one\nLine two"
    );
    assert_eq!(result["initial_stage"]["administration_revision"], 1);
    assert_eq!(result["initial_stage"]["stage_revision"], 1);
    assert_eq!(
        result["initial_stage"]["recorded_by"]["email"],
        "initial@example.com"
    );
    assert_eq!(
        result["initial_stage"]["administration_digest"],
        "11".repeat(32)
    );
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![json!([
            "create",
            "owner",
            "Current title",
            "NUC-001",
            "Line one\nLine two",
            ["Offense A", "Offense B"],
            null
        ])]
    );
}

#[tokio::test]
async fn unrevised_details_expose_no_fabricated_provenance_or_stage() {
    let workflow = Arc::new(Workflow::default());
    let response = request(&workflow, "GET", &item(), Some("baseline"), Body::empty()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let result = body(response).await;
    let current = &result["administration"];
    assert_eq!(current["revision"], 0);
    assert_eq!(current["title"], "Current title");
    assert_eq!(current["administrative_status"], "active");
    for field in ["profile", "values_digest", "changed_at", "changed_by"] {
        assert!(current.get(field).unwrap().is_null(), "{field}");
    }
    assert!(result.get("initial_stage").unwrap().is_null());
}

#[tokio::test]
async fn replacements_and_status_preserve_zero_expected_and_have_distinct_commands() {
    let workflow = Arc::new(Workflow::default());
    let mut values = replacement(0);
    values["profile"] = json!(null);
    assert_eq!(
        request(&workflow, "PUT", &item(), Some("owner"), values.to_string())
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            &workflow,
            "PUT",
            &status_path(),
            Some("owner"),
            json!({"expected_revision":0,"administrative_status":"closed"}).to_string()
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![
            json!(["replace", "owner", CASE, 0, "Current title", false]),
            json!(["status", "owner", CASE, 0, "closed"])
        ]
    );
}

#[tokio::test]
async fn staff_index_is_compact_and_passes_validated_filters_and_uuid_cursor() {
    let workflow = Arc::new(Workflow::default());
    for path in ["/api/v1/case-administrations".to_string(),format!("/api/v1/case-administrations?limit=2&after_id={CASE}&status=closed&profile=pending&title=%20A%25%20&nuc=%20NUC-1%20&judicial_case_number=CJ-1")] {
        let response=request(&workflow,"GET",&path,Some("staff"),Body::empty()).await;
        assert_eq!(response.status(),StatusCode::OK);
        let result=body(response).await;
        assert_eq!(result["next_after_id"],CASE);
        assert_eq!(result["cases"][0]["profile_status"],"complete");
        let row=&result["cases"][0];
        for field in ["profile","offenses","general_information","changed_by","values_digest"] {assert!(row.get(field).is_none(),"{field}");}
    }
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![
            json!(["list", "staff", 50, null, "Active", "All", null, null, null]),
            json!(["list", "staff", 2, CASE, "Closed", "Pending", "A%", "NUC-1", "CJ-1"])
        ]
    );
}

#[tokio::test]
async fn history_retains_positive_revision_provenance_and_exclusive_cursor() {
    let workflow = Arc::new(Workflow::default());
    let response = request(
        &workflow,
        "GET",
        &format!("{}/history?limit=2&before_revision=4", item()),
        Some("staff"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let result = body(response).await;
    assert_eq!(result["revisions"][0]["revision"], 3);
    assert_eq!(result["revisions"][0]["changed_by"]["id"], ACTOR);
    assert_eq!(result["next_before_revision"], 3);
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![json!(["history", "staff", CASE, 2, 4])]
    );
}

#[tokio::test]
async fn failures_keep_stable_codes_without_internal_or_foreign_data() {
    let workflow = Arc::new(Workflow::default());
    for (token, status, code) in [
        ("expired", 401, "invalid_session"),
        ("forbidden", 403, "permission_denied"),
        ("hidden", 404, "case_not_found"),
        ("conflict", 409, "case_revision_conflict"),
        ("exhausted", 409, "case_revision_exhausted"),
        ("closed", 409, "case_closed"),
        ("duplicate", 409, "case_identifier_conflict"),
        ("profile-required", 409, "case_profile_required"),
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
