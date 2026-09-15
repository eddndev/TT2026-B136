mod case_stage_support;
use axum::http::StatusCode;
use case_stage_support::*;
use serde_json::Value;
use std::sync::Arc;

#[tokio::test]
async fn workflow_failures_are_stable_and_do_not_leak_internal_details() {
    for (token, status, code) in [
        ("expired", 401, "invalid_session"),
        ("forbidden", 403, "permission_denied"),
        ("hidden", 404, "case_not_found"),
        ("conflict", 409, "case_stage_conflict"),
        ("required", 409, "case_stage_required"),
        ("exhausted", 409, "case_stage_revision_exhausted"),
        ("rejected", 409, "case_stage_transition_rejected"),
        ("incomplete", 409, "case_stage_profile_incomplete"),
        ("changed", 409, "stage_support_changed"),
        ("digest", 409, "stage_support_digest_mismatch"),
        ("closed", 409, "case_closed"),
        ("large", 422, "stage_support_too_large"),
        ("format", 422, "stage_support_format_rejected"),
        ("limit", 422, "stage_support_validation_limit"),
        ("failed", 500, "internal_error"),
    ] {
        for (method, suffix, value) in [
            ("GET", "", Value::Null),
            ("GET", "/history", Value::Null),
            ("POST", "/adoption", adoption()),
            ("POST", "/transitions", intermediate()),
        ] {
            let response = call(Arc::new(Workflow::default()), method, suffix, token, value).await;
            assert_eq!(
                response.status(),
                StatusCode::from_u16(status).unwrap(),
                "{method} {suffix} {token}"
            );
            assert_eq!(response.headers()["cache-control"], "no-store");
            let response = body(response).await;
            assert_eq!(response["error"]["code"], code);
            assert!(!response.to_string().contains("secret-password"));
        }
    }
}
