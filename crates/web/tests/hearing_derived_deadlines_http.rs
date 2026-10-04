mod deadline_http_support;
mod hearing_derived_deadline_http_support;
#[allow(dead_code)]
mod hearing_result_support;
use application::{deadlines::DeadlineChange, hearing_results::HearingResultChange};
use domain::{deadline_triggers::TriggerSourceRef, procedural_facts::FactDeclaration};
use hearing_derived_deadline_http_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn prepare_passes_both_exact_commands_without_a_persisted_source() {
    let workflow = Arc::new(Workflow::default());
    let (status, body) = prepare(workflow.clone(), command()).await;
    assert_eq!(status, 422, "{body}");
    let calls = workflow.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let (token, scope, command, expected) = &calls[0];
    assert_eq!(token, "owner");
    assert_eq!(*scope, case());
    assert_eq!(*expected, None);
    let HearingResultChange::Record {
        values,
        anchor_revision,
        ..
    } = &command.result.change
    else {
        panic!("record expected")
    };
    assert_eq!(anchor_revision.get(), 2);
    assert_eq!(values.summary().as_str(), "Declared session\nSummary");
    let (deadline, policies) = command.deadline.clone().into_parts();
    assert_eq!(
        policies,
        Some(deadline_http_support::records::tracking_policies())
    );
    let DeadlineChange::Register { definition } = deadline.change else {
        panic!("register expected")
    };
    let FactDeclaration::Known(TriggerSourceRef::HearingResult(source)) =
        definition.input.selection.source
    else {
        panic!("exact hearing result expected")
    };
    assert_eq!(source.result_id, command.result.result_id);
    assert_eq!(source.hearing_id, command.result.hearing_id);
    assert_eq!(source.revision.get(), 1);
    assert_eq!(source.agreement_id, None);
}

#[tokio::test]
async fn absent_bearer_precedes_json_and_size_errors() {
    for body in ["not json".into(), " ".repeat(1024 * 1024 + 1)] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(workflow.clone(), "/prepare", None, body, &[]).await;
        assert_eq!(status, 401, "{body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn path_and_selected_source_mismatches_never_reach_workflow() {
    for (pointer, value) in [
        ("/case_id", json!(HEARING)),
        ("/result/hearing_id", json!(RESULT)),
        (
            "/deadline/change/definition/input/selection/case_id",
            json!(RESULT),
        ),
        (
            "/deadline/change/definition/input/selection/source/value/hearing_id",
            json!(RESULT),
        ),
        (
            "/deadline/change/definition/input/selection/source/value/result_id",
            json!(HEARING),
        ),
        (
            "/deadline/change/definition/input/selection/source/value/revision",
            json!(2),
        ),
        (
            "/deadline/change/definition/input/selection/source",
            json!({"kind":"unknown","reason":"Not established"}),
        ),
        (
            "/deadline/change/definition/input/selection/source",
            json!({"kind":"known","value":{"family":"resolution","id":RESULT,"revision":1}}),
        ),
    ] {
        let mut input = command();
        *input.pointer_mut(pointer).unwrap() = value;
        if input["deadline"]["change"]["definition"]["input"]["selection"]["source"]["kind"]
            == "unknown"
        {
            input["deadline"]["change"]["tracking"]["source"] = json!("undetermined");
        }
        let workflow = Arc::new(Workflow::default());
        let (status, body) = prepare(workflow.clone(), input).await;
        assert_eq!(status, 400, "{pointer}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn only_record_and_register_are_accepted() {
    for pointer in ["/result/change", "/deadline/change"] {
        let mut input = command();
        *input.pointer_mut(pointer).unwrap() = if pointer.starts_with("/result") {
            json!({"action":"withdraw","expected_revision":1,"reason":"Declared withdrawal"})
        } else {
            json!({"action":"retire","expected_revision":1,"reason":"Declared retirement"})
        };
        let workflow = Arc::new(Workflow::default());
        let (status, body) = prepare(workflow.clone(), input).await;
        assert_eq!(status, 400, "{body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn application_errors_keep_status_and_hide_internal_details() {
    for (failure, expected, code) in [
        ("session", 401, "invalid_session"),
        ("permission", 403, "permission_denied"),
        ("case", 404, "case_not_found"),
        ("closed", 409, "case_closed"),
        ("source", 404, "hearing_result_reference_not_found"),
        ("operation", 409, "deadline_operation_conflict"),
        ("mismatch", 409, "deadline_submission_mismatch"),
        ("invalid", 422, "invalid_deadline"),
        ("stored", 500, "internal_error"),
        ("port", 500, "internal_error"),
    ] {
        for suffix in ["/prepare", "/submit"] {
            let workflow = Arc::new(Workflow {
                failure: Some(failure),
                ..Workflow::default()
            });
            let input = if suffix == "/submit" {
                submission()
            } else {
                command()
            };
            let (status, body) = request(
                workflow,
                suffix,
                Some("owner"),
                input.to_string(),
                &["application/json"],
            )
            .await;
            assert_eq!(status, expected, "{failure}: {body}");
            assert_eq!(body["error"]["code"], code);
            assert!(!body.to_string().contains("secret"));
        }
    }
}

#[path = "hearing_derived_deadline_http_support/boundaries.rs"]
mod boundaries;
#[path = "hearing_derived_deadline_http_support/responses.rs"]
mod responses;
