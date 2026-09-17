mod deadline_http_support;
use application::deadlines::*;
use deadline_http_support::*;
use domain::{cases::CaseId, procedural_facts::FactText};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn prepared_result_and_exact_detail_preserve_structured_calculation_and_links() {
    let detail = records::fixture();
    let workflow = Arc::new(Workflow {
        return_draft: true,
        ..Default::default()
    });
    *workflow.response.lock().unwrap() = Some(detail.clone());
    let (status, draft) = prepare(workflow.clone(), command()).await;
    assert_eq!(status, 200, "{draft}");
    assert_eq!(draft["command"], command());
    assert_eq!(draft["result_revision"], 1);
    assert_eq!(
        draft["calculation"]["result"]["arithmetic"]["outcome"]["kind"],
        "civil_candidate"
    );
    assert_eq!(
        draft["calculation"]["result"]["blocks"],
        json!([{"kind":"civil_cutoff_missing"}])
    );
    assert!(draft["calculation"]["result"]["due_at"].is_null());
    assert_eq!(
        draft["calculation"]["material"]["source"]["reference"]["id"],
        SOURCE
    );
    assert_eq!(
        draft["calculation"]["material"]["source_head"]["reference"]["revision"],
        1
    );
    assert_eq!(
        draft["calculation"]["profile"]["href"],
        format!("/api/v1/cases/{CASE}/deadline-profiles/{PROFILE}/revisions/1")
    );
    let (status, row) = request(
        workflow,
        "GET",
        &format!("{BASE}/{ID}/revisions/1"),
        Some("paralegal"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 200, "{row}");
    assert_eq!(row["calculation"], draft["calculation"]);
    assert_eq!(row["definition"], command()["change"]["definition"]);
    assert_eq!(row["receipt"]["submission_digest"], digest().to_hex());
}
#[tokio::test]
async fn all_submission_routes_return_created_and_preserve_the_confirmed_operation() {
    for (action, method, suffix) in [
        ("register", "POST", ""),
        ("correct", "PUT", "/ID"),
        ("set_attention", "POST", "/ID/attention"),
        ("retire", "POST", "/ID/retirement"),
    ] {
        let mut detail = records::fixture();
        let mut value = command();
        if action != "register" {
            detail.revision = DeadlineRevision::new(2).unwrap();
            detail.receipt.expected_revision = 1;
            detail.reason = Some(FactText::new("Declared change").unwrap());
            value["change"]["expected_revision"] = json!(1);
            value["change"]["reason"] = json!("Declared change");
            value["change"]["action"] = json!(action);
        }
        detail.receipt.action = match action {
            "register" => DeadlineAction::Register,
            "correct" => DeadlineAction::Correct,
            "set_attention" => DeadlineAction::SetAttention,
            _ => DeadlineAction::Retire,
        };
        if action == "set_attention" || action == "retire" {
            value["change"]
                .as_object_mut()
                .unwrap()
                .remove("definition");
        }
        if action == "set_attention" {
            value["change"]["attention"] = json!({"status":"pending"});
        }
        if action == "retire" {
            detail.status = DeadlineStatus::Retired;
        }
        let workflow = Arc::new(Workflow::default());
        *workflow.response.lock().unwrap() = Some(detail);
        let path = format!("{BASE}{}", suffix.replace("ID", ID));
        let (status, row) = request(
            workflow,
            method,
            &path,
            Some("litigator"),
            Some(
                json!({"command":value,"expected_submission_digest":digest().to_hex()}).to_string(),
            ),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 201, "{action}: {row}");
        assert_eq!(row["receipt"]["action"], action);
        assert_eq!(row["receipt"]["operation_id"], ID);
    }
}
#[tokio::test]
async fn list_and_history_remain_lightweight_and_paginated() {
    let workflow = Arc::new(Workflow::default());
    *workflow.response.lock().unwrap() = Some(records::fixture());
    let (status, page) = request(workflow.clone(), "GET", BASE, Some("owner"), None, &[]).await;
    assert_eq!(status, 200, "{page}");
    assert_eq!(page["deadlines"][0]["blocked"], true);
    assert!(page["deadlines"][0].get("calculation").is_none());
    let (status, page) = request(
        workflow,
        "GET",
        &format!("{BASE}/{ID}/history?limit=1"),
        Some("owner"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 200, "{page}");
    assert_eq!(page["revisions"][0]["revision"], 1);
    assert!(page["revisions"][0].get("definition").is_none());
    assert_eq!(page["next_before_revision"], json!(null));
}
#[tokio::test]
async fn foreign_or_mismatched_workflow_responses_are_rejected_before_projection() {
    for mutation in 0..6 {
        let mut detail = records::fixture();
        match mutation {
            0 => detail.case_id = CaseId::new(),
            1 => detail.id = DeadlineId::new(),
            2 => detail.revision = DeadlineRevision::new(2).unwrap(),
            3 => detail.calculation.material.case_id = CaseId::new(),
            4 => {
                detail.calculation.profile.revision =
                    application::deadline_profiles::DeadlineProfileRevision::new(2).unwrap()
            }
            _ => detail.responsible.id = domain::identity::UserId::new(),
        }
        let workflow = Arc::new(Workflow::default());
        *workflow.response.lock().unwrap() = Some(detail);
        let (status, row) = request(
            workflow,
            "GET",
            &format!("{BASE}/{ID}/revisions/1"),
            Some("owner"),
            None,
            &[],
        )
        .await;
        assert_eq!(status, 500, "mutation {mutation}: {row}");
        assert_eq!(row["error"]["code"], "internal_error");
    }
}
