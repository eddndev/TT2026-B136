mod procedural_resource_http_support;
use procedural_resource_http_support::*;
use serde_json::json;
use std::sync::Arc;
#[tokio::test]
async fn prepares_all_six_explicit_commands_and_submits_through_matching_routes() {
    let workflow = Arc::new(Workflow::default());
    for (action, method, suffix) in [
        ("register", "POST", ""),
        ("correct", "PUT", ID),
        ("record_act", "POST", &format!("{ID}/acts")),
        ("correct_act", "PUT", &format!("{ID}/acts/{ACT}")),
        ("archive", "POST", &format!("{ID}/archive")),
        ("reactivate", "POST", &format!("{ID}/reactivation")),
    ] {
        let command = command_json(action);
        let (status, body) = request(
            workflow.clone(),
            "POST",
            &format!("{}/prepare", base()),
            "ok",
            Some(command.to_string()),
        )
        .await;
        assert_eq!(status, 200, "{action}: {body}");
        assert_eq!(body["command"], command);
        assert_eq!(body["recorded_by"]["email"], "actor@example.test");
        assert_eq!(body["sources"]["resolution"]["revision"], 2);
        assert_eq!(body["sources"]["resolution"]["status"], "withdrawn");
        let path = if suffix.is_empty() {
            base()
        } else {
            format!("{}/{suffix}", base())
        };
        let (status, body) = request(
            workflow.clone(),
            method,
            &path,
            "ok",
            Some(submission(command).to_string()),
        )
        .await;
        assert_eq!(status, 201, "{action}: {body}");
        assert_eq!(body["receipt"]["action"], action);
        assert_eq!(
            body["status"],
            if action == "archive" {
                "archived"
            } else {
                "active"
            }
        );
        assert_eq!(body["recorded_at"], "1970-01-01T00:00:00Z");
        assert_eq!(
            body["act"].is_null(),
            !matches!(action, "record_act" | "correct_act")
        );
    }
    assert_eq!(workflow.commands.lock().unwrap().len(), 12);
}
#[tokio::test]
async fn reads_collection_head_exact_revision_and_history_without_replacing_refs() {
    let workflow = Arc::new(Workflow::default());
    for (path, key) in [
        (base(), "resources"),
        (
            format!("{}/{ID}/history?limit=2&before_revision=8", base()),
            "revisions",
        ),
    ] {
        let (status, body) = request(workflow.clone(), "GET", &path, "ok", None).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body[key][0]["values"]["resolution"]["revision"], 2);
        assert_eq!(body["has_more"], false);
    }
    for suffix in [ID.to_string(), format!("{ID}/revisions/5")] {
        let (status, body) = request(
            workflow.clone(),
            "GET",
            &format!("{}/{suffix}", base()),
            "ok",
            None,
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["id"], ID);
        if suffix.contains("revisions") {
            assert_eq!(body["revision"], 5);
            assert_eq!(body["receipt"]["previous"]["revision"], 4);
        }
    }
}
#[tokio::test]
async fn historical_response_scope_and_source_mismatches_are_never_projected() {
    for token in [
        "foreign",
        "wrong_revision",
        "wrong_source",
        "wrong_support",
        "wrong_previous",
        "unsafe_name",
    ] {
        let (status, body) = request(
            Arc::new(Workflow::default()),
            "GET",
            &format!("{}/{ID}/revisions/2", base()),
            token,
            None,
        )
        .await;
        assert_eq!(status, 500, "{token}: {body}");
        assert_eq!(body["error"]["code"], "internal_error");
    }
}
#[tokio::test]
async fn application_errors_keep_conflicts_denials_and_private_failures_distinct() {
    for (token, status, code) in [
        ("forbidden", 403, "permission_denied"),
        ("expired", 401, "invalid_session"),
        ("closed", 409, "case_closed"),
        ("missing", 404, "procedural_resource_not_found"),
        ("conflict", 409, "procedural_resource_revision_conflict"),
        ("operation", 409, "procedural_resource_operation_conflict"),
        ("archived", 409, "procedural_resource_archived"),
        ("unchanged", 409, "procedural_resource_state_unchanged"),
        ("mismatch", 409, "procedural_resource_submission_mismatch"),
        ("internal", 500, "internal_error"),
    ] {
        let (actual, body) = request(
            Arc::new(Workflow::default()),
            "GET",
            &format!("{}/{ID}", base()),
            token,
            None,
        )
        .await;
        assert_eq!(actual, status);
        assert_eq!(body["error"]["code"], code);
        assert!(!body.to_string().contains("private database"));
    }
}
#[tokio::test]
async fn both_resource_families_and_oral_act_keep_unknowns_and_precision() {
    let workflow = Arc::new(Workflow::default());
    let mut command = command_json("register");
    command["change"]["values"]["kind"] = json!("appeal");
    command["change"]["values"]["mode"] =
        json!({"kind":"unknown","reason":"Modality not recorded"});
    let (status, body) = request(
        workflow,
        "POST",
        &format!("{}/prepare", base()),
        "ok",
        Some(command.clone().to_string()),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["values"], command["change"]["values"]);
}
