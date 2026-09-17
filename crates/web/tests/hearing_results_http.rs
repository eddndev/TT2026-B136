#[allow(dead_code)]
mod hearing_result_support;
use hearing_result_support::*;
use serde_json::json;
use std::sync::Arc;
#[tokio::test]
async fn prepare_normalizes_values_and_preserves_exact_cancelled_anchor() {
    let w = Arc::new(Workflow::default());
    let (status, body) = request(
        w.clone(),
        "POST",
        &format!("{}/prepare", base_url()),
        Some("valid"),
        Some(command_json().to_string()),
        &["application/json"],
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["case_id"], CASE);
    assert_eq!(body["command"]["result_id"], RESULT);
    assert_eq!(body["values"]["summary"], "Declared session\nSummary");
    assert_eq!(body["anchor"]["status"], "cancelled");
    assert_eq!(body["anchor"]["revision"], 2);
    assert_eq!(body["values"]["event_time"]["precision"], "date");
    assert_eq!(w.calls.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn all_three_actions_use_matching_routes_and_return_created_receipts() {
    for (action, method, suffix, revision) in [
        ("record", "POST", "".to_string(), 1),
        ("correct", "PUT", format!("/{RESULT}"), 2),
        ("withdraw", "POST", format!("/{RESULT}/withdrawal"), 2),
    ] {
        let mut command = command_json();
        if action != "record" {
            command["change"] =
                json!({"action":action,"expected_revision":1,"reason":"Explicit reason"});
            if action == "correct" {
                command["change"]["values"] = values_json();
            }
        }
        let body = json!({"command":command,"expected_submission_digest":digest().to_hex()});
        let w = Arc::new(Workflow::default());
        let (status, response) = request(
            w.clone(),
            method,
            &format!("{}{suffix}", base_url()),
            Some("valid"),
            Some(body.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 201, "{response}");
        assert_eq!(response["revision"], revision);
        assert_eq!(response["receipt"]["action"], action);
        assert_eq!(response["hearing_id"], HEARING);
        assert_eq!(response["id"], RESULT);
        assert_eq!(w.calls.lock().unwrap().len(), 1);
    }
}
#[tokio::test]
async fn root_listing_exact_detail_and_lightweight_history_are_separate() {
    for suffix in [
        "".to_string(),
        format!("/{RESULT}"),
        format!("/{RESULT}/revisions/1"),
        format!("/{RESULT}/history"),
    ] {
        let w = Arc::new(Workflow::default());
        let (status, body) = request(
            w.clone(),
            "GET",
            &format!("{}{suffix}", base_url()),
            Some("valid"),
            None,
            &[],
        )
        .await;
        assert_eq!(status, 200, "{body}");
        if suffix.is_empty() {
            assert_eq!(body["results"][0]["id"], RESULT);
            assert!(body["results"][0].get("values").is_none());
        } else if suffix.ends_with("history") {
            assert_eq!(body["revisions"][0]["revision"], 1);
            assert!(body["revisions"][0].get("values").is_none());
            assert!(body["revisions"][0].get("attendees").is_none());
            assert_eq!(w.calls.lock().unwrap()[0][4], 10);
        } else {
            assert_eq!(body["anchor"]["status"], "cancelled");
        }
    }
}
#[tokio::test]
async fn target_source_and_case_missing_have_distinct_codes_and_internal_details_are_hidden() {
    for (token, status, code) in [
        ("missing", 404, "hearing_result_not_found"),
        ("source_missing", 404, "hearing_result_reference_not_found"),
        ("case_missing", 404, "case_not_found"),
        ("forbidden", 403, "permission_denied"),
        ("expired", 401, "invalid_session"),
        ("conflict", 409, "hearing_result_revision_conflict"),
        ("future", 422, "hearing_result_future_time"),
        ("internal", 500, "internal_error"),
    ] {
        let (actual, body) = request(
            Arc::new(Workflow::default()),
            "GET",
            &format!("{}/{RESULT}", base_url()),
            Some(token),
            None,
            &[],
        )
        .await;
        assert_eq!(actual, status, "{token}: {body}");
        assert_eq!(body["error"]["code"], code);
        assert!(!body.to_string().contains("secret source"));
    }
}
#[tokio::test]
async fn bad_queries_and_scope_action_mismatches_never_call_workflow() {
    for suffix in [
        "?limit=0".to_string(),
        "?limit=101".into(),
        "?limit=2&limit=3".into(),
        "?unknown=x".into(),
        format!("/{RESULT}/history?limit=21"),
        format!("/{RESULT}/history?before_revision=0"),
    ] {
        let w = Arc::new(Workflow::default());
        let (status, body) = request(
            w.clone(),
            "GET",
            &format!("{}{suffix}", base_url()),
            Some("valid"),
            None,
            &[],
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(w.calls.lock().unwrap().is_empty());
    }
    for mode in 0..3 {
        let mut command = command_json();
        let (method, path) = match mode {
            0 => {
                command["hearing_id"] = json!(RESULT);
                ("POST", format!("{}/prepare", base_url()))
            }
            1 => ("PUT", format!("{}/{RESULT}", base_url())),
            _ => ("POST", format!("{}/{RESULT}/withdrawal", base_url())),
        };
        let payload = if mode == 0 {
            command
        } else {
            json!({"command":command,"expected_submission_digest":digest().to_hex()})
        };
        let w = Arc::new(Workflow::default());
        let (status, body) = request(
            w.clone(),
            method,
            &path,
            Some("valid"),
            Some(payload.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(w.calls.lock().unwrap().is_empty());
    }
}
