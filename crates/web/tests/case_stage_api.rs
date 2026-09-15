mod case_stage_support;
use axum::http::StatusCode;
use case_stage_support::*;
use serde_json::{json, Value};
use std::sync::Arc;

#[tokio::test]
async fn unregistered_and_initial_have_distinct_explicit_projections() {
    let workflow = Arc::new(Workflow::default());
    let response = call(workflow.clone(), "GET", "", "empty", Value::Null).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(body(response).await, json!({"case_id":CASE,"current":null}));
    let current = body(call(workflow, "GET", "", "owner", Value::Null).await).await;
    assert_eq!(
        current["current"],
        json!({"kind":"initial","case_id":CASE,"stage_revision":1,"stage":"investigation","administration_revision":1,"administration_digest":"11".repeat(32),"recorded_at":"2023-11-14T22:13:20Z","recorded_by":{"id":"00000000-0000-0000-0000-000000000003","email":"owner@example.test"}})
    );
}
#[tokio::test]
async fn adoption_preserves_declared_precision_and_normalizes_only_domain_text() {
    let workflow = Arc::new(Workflow::default());
    let response = call(workflow.clone(), "POST", "/adoption", "owner", adoption()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let result = body(response).await;
    let current = &result["current"];
    assert_eq!(current["kind"], "change");
    assert_eq!(current["stage_revision"], 1);
    assert_eq!(current["from_stage"], Value::Null);
    assert_eq!(
        current["values"],
        json!({"kind":"adoption","stage":"intermediate","known_at":date(),"reason":"Motivo\n declarado","support":support()})
    );
    assert_eq!(
        current["supports"],
        json!([{"document_id":DOCUMENT,"version":1,"digest":"ab".repeat(32),"name":"acta.pdf","format":"pdf","policy":"pdf_docx_v1"}])
    );
    assert_eq!(workflow.calls.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn transition_preserves_instant_offset_fraction_and_exact_reference() {
    let workflow = Arc::new(Workflow::default());
    let response = call(
        workflow.clone(),
        "POST",
        "/transitions",
        "owner",
        intermediate(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let result = body(response).await;
    assert_eq!(result["current"]["stage_revision"], 2);
    assert_eq!(result["current"]["from_stage"], "investigation");
    assert_eq!(
        result["current"]["values"],
        json!({"kind":"to_intermediate","accusation_declared_at":intermediate()["accusation_declared_at"],"accusation":support(),"note":"Nota"})
    );
    assert_eq!(workflow.calls.lock().unwrap()[0]["expected"], 1);
}
#[tokio::test]
async fn trial_maps_both_roles_but_deduplicated_supports_and_optional_values() {
    let workflow = Arc::new(Workflow::default());
    let response = call(workflow.clone(), "POST", "/transitions", "owner", trial()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let result = body(response).await;
    let values = &result["current"]["values"];
    assert_eq!(result["current"]["stage"], "trial");
    assert_eq!(values["kind"], "to_trial");
    assert_eq!(values["opening_order"], support());
    assert_eq!(values["receipt_support"], support());
    assert_eq!(values["receiving_court"], "Tribunal Uno");
    assert_eq!(values["receipt_reference"], "Acuse 9");
    assert_eq!(values["note"], "Nota\nmanual");
    assert_eq!(result["current"]["supports"].as_array().unwrap().len(), 1);
    let mut minimal = trial();
    for key in ["receipt_reference", "receipt_support", "note"] {
        minimal.as_object_mut().unwrap().remove(key);
    }
    let result = body(call(workflow, "POST", "/transitions", "owner", minimal).await).await;
    assert!(result["current"]["values"]["receipt_support"].is_null());
}
#[tokio::test]
async fn history_maps_cursor_and_does_not_make_per_entry_workflow_calls() {
    let workflow = Arc::new(Workflow::default());
    for (suffix, limit, before) in [
        ("/history", 20, Value::Null),
        ("/history?limit=1&before_revision=3", 1, json!(3)),
    ] {
        let response = call(workflow.clone(), "GET", suffix, "owner", Value::Null).await;
        assert_eq!(response.status(), StatusCode::OK);
        let result = body(response).await;
        assert_eq!(result["entries"][0]["kind"], "initial");
        assert_eq!(result["has_more"], false);
        assert_eq!(result["next_before_revision"], Value::Null);
        assert_eq!(
            workflow.calls.lock().unwrap().last().unwrap(),
            &json!({"op":"history","limit":limit,"before":before})
        );
    }
    assert_eq!(workflow.calls.lock().unwrap().len(), 2);
    let result = body(call(workflow, "GET", "/history?limit=1", "page", Value::Null).await).await;
    assert_eq!(result["has_more"], true);
    assert_eq!(result["next_before_revision"], 2);
    assert_eq!(result["entries"][0]["stage_revision"], 2);
    assert_eq!(result["entries"][0]["values"]["kind"], "to_intermediate");
}
#[tokio::test]
async fn a_foreign_entry_is_an_opaque_internal_failure() {
    let response = call(
        Arc::new(Workflow::default()),
        "GET",
        "",
        "foreign",
        Value::Null,
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body(response).await["error"]["code"], "internal_error");
}
