use super::route_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn context_list_detail_exact_and_history_use_authorized_workflow() {
    let w = Arc::new(Workflow::default());
    for suffix in [
        "context",
        "",
        ID,
        &format!("{ID}/revisions/1"),
        &format!("{ID}/history"),
    ] {
        let path = format!(
            "/api/v1/cases/{CASE}/hearings{}",
            if suffix.is_empty() {
                String::new()
            } else {
                format!("/{suffix}")
            }
        );
        let (status, body) = call(w.clone(), "GET", &path, None, Some("valid")).await;
        assert_eq!(status, 200, "{path}: {body}");
    }
    assert_eq!(w.calls.lock().unwrap().len(), 5);
}
#[tokio::test]
async fn preparation_returns_normalized_command_and_commit_receipt() {
    let w = Arc::new(Workflow::default());
    let path = format!("/api/v1/cases/{CASE}/hearings");
    let (status, draft) = call(
        w.clone(),
        "POST",
        &format!("{path}/prepare"),
        Some(super::value_tests::command()),
        Some("valid"),
    )
    .await;
    assert_eq!(status, 200, "{draft}");
    assert_eq!(draft["command"]["change"]["values"]["venue"], "Room");
    assert_eq!(draft["actor_id"], actor().to_string());
    let (status,saved)=call(w,"POST",&path,Some(json!({"command":draft["command"],"expected_submission_digest":draft["submission_digest"]})),Some("valid")).await;
    assert_eq!(status, 201, "{saved}");
    assert_eq!(saved["receipt"]["submission_digest"], digest().to_hex());
    assert_eq!(saved["recorded_by"]["id"], actor().to_string());
    assert_eq!(saved["values"]["scheduled_at"], "2026-10-01T09:00:15-06:00");
}
#[tokio::test]
async fn wrong_action_or_path_does_not_reach_the_workflow() {
    let w = Arc::new(Workflow::default());
    let path = format!("/api/v1/cases/{CASE}/hearings/{ID}/cancellation");
    let body = json!({"command":super::value_tests::command(),"expected_submission_digest":digest().to_hex()});
    let (status, _) = call(w.clone(), "POST", &path, Some(body), Some("valid")).await;
    assert_eq!(status, 400);
    assert!(w.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn unauthenticated_invalid_and_forbidden_requests_keep_their_boundaries() {
    let w = Arc::new(Workflow::default());
    let path = format!("/api/v1/cases/{CASE}/hearings/{ID}");
    assert_eq!(call(w.clone(), "GET", &path, None, None).await.0, 401);
    assert!(w.calls.lock().unwrap().is_empty());
    for (token, status) in [
        ("forbidden", 403),
        ("expired", 401),
        ("missing", 404),
        ("conflict", 409),
        ("internal", 500),
    ] {
        let (actual, body) = call(w.clone(), "GET", &path, None, Some(token)).await;
        assert_eq!(actual, status);
        assert!(!body.to_string().contains("secret"));
    }
}
#[tokio::test]
async fn agenda_uses_one_global_workflow_and_rejects_partial_cursor() {
    let w = Arc::new(Workflow::default());
    let path = "/api/v1/hearings?from=2026-10-01T00:00:00Z&until=2026-10-02T00:00:00Z";
    let (status, body) = call(w.clone(), "GET", path, None, Some("valid")).await;
    assert_eq!(status, 200, "{body}");
    assert!(body["hearings"][0].get("venue").is_none());
    assert!(body["hearings"][0].get("participants").is_none());
    assert_eq!(w.calls.lock().unwrap().len(), 1);
    let (status, _) = call(
        w.clone(),
        "GET",
        &format!("{path}&after_id={ID}"),
        None,
        Some("valid"),
    )
    .await;
    assert_eq!(status, 400);
    assert_eq!(w.calls.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn detail_rejects_cross_case_or_wrong_exact_revision_projection() {
    let w = Arc::new(Workflow::default());
    let path = format!("/api/v1/cases/{CASE}/hearings/{ID}");
    let mut wrong = detail();
    wrong.snapshot.case_id = domain::cases::CaseId::new();
    *w.response.lock().unwrap() = Some(wrong);
    assert_eq!(
        call(w.clone(), "GET", &path, None, Some("valid")).await.0,
        500
    );
    *w.response.lock().unwrap() = Some(detail());
    assert_eq!(
        call(
            w,
            "GET",
            &format!("{path}/revisions/2"),
            None,
            Some("valid")
        )
        .await
        .0,
        500
    );
}

#[tokio::test]
async fn replacements_and_cancellations_return_new_revisions_and_original_context() {
    let w = Arc::new(Workflow::default());
    let path = format!("/api/v1/cases/{CASE}/hearings/{ID}");
    let mut command = super::value_tests::command();
    command["change"]["action"] = json!("replace");
    command["change"]["expected_revision"] = json!(1);
    command["change"]["reason"] = json!("New communicated date");
    command["change"]["values"]["scheduled_at"] = json!("2026-11-02T11:25:10-06:00");
    let body = json!({"command": command, "expected_submission_digest": digest().to_hex()});
    let (status, saved) = call(w.clone(), "PUT", &path, Some(body), Some("valid")).await;
    assert_eq!(status, 201, "{saved}");
    assert_eq!(saved["revision"], 2);
    assert_eq!(saved["receipt"]["action"], "replace");
    assert_eq!(saved["values"]["scheduled_at"], "2026-11-02T11:25:10-06:00");
    command["change"] = json!({"action": "cancel", "expected_revision": 1, "reason": "Organizational cancellation"});
    let body = json!({"command": command, "expected_submission_digest": digest().to_hex()});
    let (status, saved) = call(
        w,
        "POST",
        &format!("{path}/cancellation"),
        Some(body),
        Some("valid"),
    )
    .await;
    assert_eq!(status, 201, "{saved}");
    assert_eq!(saved["status"], "cancelled");
    assert_eq!(
        saved["receipt"]["expected_context"],
        serde_json::Value::Null
    );
    assert_eq!(saved["scheduling_context"]["stage_revision"], 1);
}

#[tokio::test]
async fn invalid_agenda_queries_never_invoke_global_reads() {
    let w = Arc::new(Workflow::default());
    for query in [
        "from=2026-10-01T00:00:00Z",
        "from=2026-10-02T00:00:00Z&until=2026-10-01T00:00:00Z",
        "from=2026-10-01T00:00:00Z&until=2027-10-03T00:00:00Z",
        "from=2026-10-01T00:00:00-06:00&until=2026-10-02T00:00:00Z",
        "from=2026-10-01T00:00:00Z&until=2026-10-02T00:00:00Z&limit=101",
        "from=2026-10-01T00:00:00Z&until=2026-10-02T00:00:00Z&status=completed",
        "from=2026-10-01T00:00:00Z&until=2026-10-02T00:00:00Z&limit=20&limit=10",
        "from=2026-10-01T00:00:00Z&until=2026-10-02T00:00:00Z&unknown=value",
    ] {
        let (status, body) = call(
            w.clone(),
            "GET",
            &format!("/api/v1/hearings?{query}"),
            None,
            Some("valid"),
        )
        .await;
        assert_eq!(status, 400, "{query}: {body}");
    }
    assert!(w.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn unsupported_dates_and_zero_support_versions_are_client_errors() {
    let w = Arc::new(Workflow::default());
    let path = format!("/api/v1/cases/{CASE}/hearings/prepare");
    let mut command = super::value_tests::command();
    command["change"]["values"]["scheduled_at"] = json!("2026-02-29T12:00:00Z");
    let (status, body) = call(w.clone(), "POST", &path, Some(command), Some("valid")).await;
    assert_eq!(status, 422);
    assert_eq!(body["error"]["code"], "invalid_hearing_value");
    let mut command = super::value_tests::command();
    command["change"]["values"]["kind"] = json!("sentencing");
    command["change"]["values"]["conviction_basis"] = json!({"statement": "Declared finding", "support": {"document_id": ID, "version": 0, "digest": digest().to_hex()}});
    let (status, body) = call(w.clone(), "POST", &path, Some(command), Some("valid")).await;
    assert_eq!(status, 400, "{body}");
    assert_eq!(body["error"]["code"], "invalid_document_version");
    assert!(w.calls.lock().unwrap().is_empty());
}
