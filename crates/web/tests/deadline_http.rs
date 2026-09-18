mod deadline_http_support;
use application::deadlines::*;
use deadline_http_support::*;
use domain::{deadline_triggers::TriggerSourceRef, procedural_facts::FactDeclaration};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn list_preserves_case_scope_and_explicit_filters_before_pagination() {
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        workflow.clone(),
        "GET",
        &format!("{BASE}?limit=1&status=all&after_id={ID}"),
        Some("paralegal"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({"case_id":CASE,"deadlines":[],"has_more":false,"next_after_id":null})
    );
    assert_eq!(
        workflow.calls.lock().unwrap()[0],
        json!(["paralegal", CASE, ["list", 1, ID, null]])
    );
}

#[tokio::test]
async fn typed_prepare_passes_exact_input_and_normalizes_human_text_only() {
    let workflow = Arc::new(Workflow::default());
    let mut value = command();
    value["change"]["definition"]["title"] = json!("  Declared period  ");
    let (status, body) = prepare(workflow.clone(), value).await;
    assert_eq!(status, 422, "{body}");
    let guard = workflow.command.lock().unwrap();
    let command = guard
        .as_ref()
        .expect("valid command must reach the workflow");
    let DeadlineChange::Register { definition } = &command.change else {
        panic!("register expected")
    };
    assert_eq!(definition.title.as_str(), "Declared period");
    assert_eq!(definition.input.selection.case_id, case());
    assert_eq!(definition.input.ordered_quantity, None);
    assert_eq!(definition.input.calendar, None);
    let FactDeclaration::Known(TriggerSourceRef::Resolution(reference)) =
        definition.input.selection.source
    else {
        panic!("resolution reference expected")
    };
    assert_eq!(reference.id.to_string(), SOURCE);
    assert_eq!(reference.revision.get(), 1);
}

#[tokio::test]
async fn mutation_routes_reject_cross_action_identity_and_case_before_workflow() {
    for (method, path, value) in [
        ("PUT", format!("{BASE}/{ID}"), submission()),
        ("POST", format!("{BASE}/{ID}/attention"), submission()),
        ("POST", format!("{BASE}/{ID}/retirement"), submission()),
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            method,
            &path,
            Some("owner"),
            Some(value.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    let workflow = Arc::new(Workflow::default());
    let mut value = command();
    value["change"]["definition"]["input"]["selection"]["case_id"] = json!(ID);
    let (status, body) = prepare(workflow.clone(), value).await;
    assert_eq!(status, 400, "{body}");
    assert!(workflow.calls.lock().unwrap().is_empty());
}
