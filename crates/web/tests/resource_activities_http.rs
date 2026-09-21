mod resource_activity_http_support;
use resource_activity_http_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn prepares_and_confirms_exact_link_then_organizational_unlink() {
    let workflow = Arc::new(Workflow::default());
    for (action, suffix) in [("link", ""), ("unlink", "/unlink")] {
        let command = command_json(action, "hearing");
        let (status, draft) = request(
            workflow.clone(),
            "POST",
            &format!("{}/prepare", base()),
            "ok",
            Some(command.to_string()),
        )
        .await;
        assert_eq!(status, 200, "{draft}");
        assert_eq!(draft["case_id"], CASE);
        assert_eq!(draft["resource_id"], RESOURCE);
        assert_eq!(draft["command"], command);
        let path = if action == "link" {
            base()
        } else {
            format!("{}/{ID}{suffix}", base())
        };
        let (status, row) = request(
            workflow.clone(),
            "POST",
            &path,
            "ok",
            Some(submission(command).to_string()),
        )
        .await;
        assert_eq!(status, 201, "{row}");
        assert_eq!(row["case_id"], CASE);
        assert_eq!(row["resource_id"], RESOURCE);
        assert_eq!(row["id"], ID);
        assert_eq!(row["receipt"]["action"], action);
        assert_eq!(row["selection"]["resource"]["revision"], 2);
        assert_eq!(row["selection"]["target"]["revision"], 1);
        assert!(row.get("current_target").is_none());
    }
    assert_eq!(workflow.commands.lock().unwrap().len(), 4);
}

#[tokio::test]
async fn exact_association_keeps_old_target_while_current_head_is_separate() {
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        workflow.clone(),
        "GET",
        &format!("{}/{ID}/revisions/1", base()),
        "ok",
        None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["association"]["revision"], 1);
    assert_eq!(body["checked_at"]["offset_seconds"], 0);
    assert_eq!(body["checked_at"]["nanosecond"], 0);
    assert_eq!(body["association"]["selection"]["target"]["revision"], 1);
    assert_eq!(body["current_target"]["kind"], "hearing");
    assert_eq!(body["current_target"]["record"]["revision"], 2);
    assert_eq!(body["current_target"]["record"]["case_id"], CASE);
    let (status, history) = request(
        workflow,
        "GET",
        &format!("{}/{ID}/history?limit=2&before_revision=3", base()),
        "ok",
        None,
    )
    .await;
    assert_eq!(status, 200, "{history}");
    assert_eq!(
        history["revisions"][0]["selection"]["target"]["revision"],
        1
    );
    assert!(history["revisions"][0].get("current_target").is_none());
}

#[tokio::test]
async fn deadline_projection_preserves_retirement_without_historical_date_fallback() {
    let (status, body) = request(
        Arc::new(Workflow::default()),
        "GET",
        &format!("{}/{ID}", base()),
        "deadline",
        None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["current_target"]["kind"], "deadline");
    assert_eq!(body["association"]["selection"]["target"]["revision"], 1);
    assert_eq!(body["current_target"]["record"]["revision"], 2);
    assert_eq!(body["current_target"]["record"]["status"], "retired");
    assert!(body["current_target"]["record"]["operational"]["due_at"].is_null());
    assert!(body["current_target"]["record"]["calculation"]["result"]["due_at"].is_object());
}

#[tokio::test]
async fn filtered_page_has_url_scope_and_bounded_continuation() {
    let workflow = Arc::new(Workflow::default());
    let path = format!(
        "{}?limit=2&kind=hearing&status=linked&after_id={BEFORE}",
        base()
    );
    let (status, body) = request(workflow.clone(), "GET", &path, "ok", None).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["associations"][0]["association"]["resource_id"],
        RESOURCE
    );
    assert_eq!(body["has_more"], false);
    assert_eq!(body["next_after_id"], json!(null));
    let calls = workflow.calls.lock().unwrap();
    assert_eq!(calls[0]["case_id"], CASE);
    assert_eq!(calls[0]["resource_id"], RESOURCE);
    assert_eq!(calls[0]["limit"], 2);
    assert_eq!(calls[0]["after_id"], BEFORE);
}

#[tokio::test]
async fn wrong_scope_reference_revision_and_current_target_never_escape_projection() {
    for token in [
        "foreign_case",
        "foreign_resource",
        "foreign_association",
        "wrong_revision",
        "wrong_target",
        "wrong_current",
    ] {
        let (status, body) = request(
            Arc::new(Workflow::default()),
            "GET",
            &format!("{}/{ID}/revisions/1", base()),
            token,
            None,
        )
        .await;
        assert_eq!(status, 500, "{token}: {body}");
        assert_eq!(body["error"]["code"], "internal_error");
    }
}

#[tokio::test]
async fn role_denial_revocation_closure_and_conflicts_remain_distinct_and_private() {
    for (token, status, code) in [
        ("client", 403, "permission_denied"),
        ("paralegal", 403, "permission_denied"),
        ("revoked", 403, "permission_denied"),
        ("expired", 401, "invalid_session"),
        ("closed", 409, "case_closed"),
        ("missing", 404, "resource_activity_not_found"),
        ("conflict", 409, "resource_activity_revision_conflict"),
        (
            "resource_conflict",
            409,
            "resource_activity_resource_revision_conflict",
        ),
        ("operation", 409, "resource_activity_operation_conflict"),
        ("mismatch", 409, "resource_activity_submission_mismatch"),
        ("internal", 500, "internal_error"),
    ] {
        let (actual, body) = request(
            Arc::new(Workflow::default()),
            "POST",
            &base(),
            token,
            Some(submission(command_json("link", "hearing")).to_string()),
        )
        .await;
        assert_eq!(actual, status, "{token}: {body}");
        assert_eq!(body["error"]["code"], code);
        assert!(!body.to_string().contains("private database"));
    }
}

#[tokio::test]
async fn all_staff_roles_read_history_through_the_authorized_workflow() {
    for token in ["owner", "litigator", "paralegal"] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            "GET",
            &format!("{}/{ID}/history", base()),
            token,
            None,
        )
        .await;
        assert_eq!(status, 200, "{token}: {body}");
        assert_eq!(workflow.calls.lock().unwrap()[0]["method"], "history");
    }
}

#[tokio::test]
async fn optional_act_keeps_its_own_containing_revision_and_exact_support() {
    let workflow = Arc::new(Workflow::default());
    let mut command = command_json("link", "hearing");
    command["change"]["act"] = json!({"id":ACT,"revision":1,
        "resource_revision":3,"capture_digest":"07".repeat(32)});
    let (status, draft) = request(
        workflow.clone(),
        "POST",
        &format!("{}/prepare", base()),
        "ok",
        Some(command.to_string()),
    )
    .await;
    assert_eq!(status, 200, "{draft}");
    assert_eq!(draft["command"], command);
    assert_eq!(draft["sources"]["resource"]["revision"], 2);
    assert_eq!(draft["sources"]["act"]["revision"], 3);
    assert_eq!(draft["sources"]["act"]["act"]["id"], ACT);
    assert_eq!(draft["sources"]["act"]["act"]["revision"], 1);
    assert_eq!(
        draft["sources"]["act"]["act"]["values"]["mode"]["value"],
        "oral"
    );
    assert_eq!(draft["sources"]["act"]["act"]["supports"][0]["version"], 3);
    let (status, row) = request(
        workflow.clone(),
        "POST",
        &base(),
        "ok",
        Some(submission(command).to_string()),
    )
    .await;
    assert_eq!(status, 201, "{row}");
    assert_eq!(row["sources"], draft["sources"]);
    assert_eq!(row["selection"]["act"], draft["selection"]["act"]);
    let (status, view) = request(
        workflow.clone(),
        "GET",
        &format!("{}/{ID}/revisions/1", base()),
        "act",
        None,
    )
    .await;
    assert_eq!(status, 200, "{view}");
    assert_eq!(view["association"]["sources"]["act"], row["sources"]["act"]);
    let (status, denied) = request(
        workflow,
        "GET",
        &format!("{}/{ID}", base()),
        "wrong_act_capture",
        None,
    )
    .await;
    assert_eq!(status, 500, "{denied}");
    assert_eq!(denied["error"]["code"], "internal_error");
}
