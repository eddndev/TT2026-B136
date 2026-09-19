mod deadline_http_support;
use application::deadlines::*;
use deadline_http_support::*;
use domain::{
    deadline_triggers::*, procedural_facts::FactDeclaration,
    procedural_time::DeclaredProceduralPrecision,
};
use serde_json::{json, Value};
use std::sync::Arc;

#[tokio::test]
async fn exact_notification_and_hearing_references_do_not_infer_other_sources() {
    for source in [
        json!({"family":"notification","id":SOURCE,"revision":3,"resolution":{"id":ID,"revision":2}}),
        json!({"family":"hearing_result","hearing_id":ID,"result_id":SOURCE,"revision":4,"agreement_id":ID}),
    ] {
        let workflow = Arc::new(Workflow::default());
        let mut value = command();
        value["change"]["definition"]["input"]["selection"]["source"]["value"] = source.clone();
        let (_, body) = prepare(workflow.clone(), value).await;
        let command = workflow
            .command
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| panic!("parsed command expected: {body}"));
        let DeadlineChange::Register { definition } = &command.change else {
            panic!("register expected")
        };
        match definition.input.selection.source {
            FactDeclaration::Known(TriggerSourceRef::Notification {
                id,
                revision,
                resolution,
            }) => {
                assert_eq!(id.to_string(), SOURCE);
                assert_eq!(revision.get(), 3);
                assert_eq!(resolution.id.to_string(), ID);
                assert_eq!(resolution.revision.get(), 2);
            }
            FactDeclaration::Known(TriggerSourceRef::HearingResult(r)) => {
                assert_eq!(r.hearing_id.to_string(), ID);
                assert_eq!(r.result_id.to_string(), SOURCE);
                assert_eq!(r.revision.get(), 4);
                assert_eq!(r.agreement_id.unwrap().to_string(), ID);
            }
            _ => panic!("exact selected family expected"),
        }
    }
}

#[tokio::test]
async fn explicit_qualified_times_and_attention_preserve_precision_and_optional_offset() {
    for (at, precision) in [
        (
            json!({"precision":"unknown"}),
            DeclaredProceduralPrecision::Unknown,
        ),
        (
            json!({"precision":"date","year":2026,"month":1,"day":9,"offset_seconds":null}),
            DeclaredProceduralPrecision::Date,
        ),
        (
            json!({"precision":"minute","year":2026,"month":1,"day":9,"hour":10,"minute":12,"offset_seconds":0}),
            DeclaredProceduralPrecision::Minute,
        ),
        (
            json!({"precision":"second","year":2026,"month":1,"day":9,"hour":10,"minute":12,"second":0,"offset_seconds":-21600}),
            DeclaredProceduralPrecision::Second,
        ),
    ] {
        let workflow = Arc::new(Workflow::default());
        let mut value = command();
        value["change"]["definition"]["input"]["selection"]["qualification"] = json!({"purpose":"ordered_period_start","at":at,"statement":"Declared start","locator":"Page 2"});
        let (_, body) = prepare(workflow.clone(), value).await;
        let command = workflow
            .command
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| panic!("parsed command expected: {body}"));
        let DeadlineChange::Register { definition } = &command.change else {
            panic!("register expected")
        };
        let captured = definition
            .input
            .selection
            .qualification
            .as_ref()
            .unwrap()
            .at;
        assert_eq!(captured.precision(), precision);
        assert_eq!(
            captured.local_second().map(u32::from),
            at["second"].as_u64().map(|v| v as u32)
        );
        assert_eq!(
            captured.offset().map(|v| v.whole_seconds()),
            at["offset_seconds"].as_i64().map(|v| v as i32)
        );
        let workflow = Arc::new(Workflow::default());
        let mut value = command_json_attention(at);
        let (_, body) = prepare(workflow.clone(), value.take()).await;
        let command = workflow.command.lock().unwrap().clone();
        let Some(DeadlineCommand {
            change:
                DeadlineChange::SetAttention {
                    attention: DeadlineAttention::Recorded { occurred_at, .. },
                    ..
                },
            ..
        }) = command.as_ref()
        else {
            panic!("attention expected: {body}")
        };
        assert_eq!(occurred_at.precision(), precision);
    }
}
fn command_json_attention(at: Value) -> Value {
    json!({"operation_id":ID,"deadline_id":ID,"change":{"action":"set_attention","expected_revision":1,
        "attention":{"status":"recorded","occurred_at":at,"statement":"Declared action","locator":"Receipt"},"reason":"New declaration"}})
}

#[tokio::test]
async fn unknown_declarations_calendar_and_ordered_quantity_remain_explicit() {
    let workflow = Arc::new(Workflow::default());
    let mut value = command();
    let input = &mut value["change"]["definition"]["input"];
    input["selection"]["source"] = json!({"kind":"unknown","reason":"Not stated"});
    input["calendar"] = json!({"id":ID,"revision":2});
    input["ordered_quantity"] = json!(4294967295u32);
    input["qualification"]["scope_applies"] = json!({"kind":"unknown","reason":"Scope pending"});
    value["change"]["tracking"]["source"] = json!("undetermined");
    value["change"]["tracking"]["calendar"] = json!("follow");
    let (_, body) = prepare(workflow.clone(), value).await;
    let command = workflow
        .command
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| panic!("parsed command expected: {body}"));
    let DeadlineChange::Register { definition } = &command.change else {
        panic!("register expected")
    };
    assert!(matches!(
        definition.input.selection.source,
        FactDeclaration::Unknown(_)
    ));
    assert_eq!(definition.input.calendar.unwrap().revision.get(), 2);
    assert_eq!(definition.input.ordered_quantity.unwrap().get(), u32::MAX);
    assert!(matches!(
        definition.input.qualification.scope_applies,
        FactDeclaration::Unknown(_)
    ));
}

#[tokio::test]
async fn named_objects_unknown_fields_and_duplicate_keys_are_rejected_before_workflow() {
    for pointer in [
        "",
        "/change",
        "/change/definition",
        "/change/definition/profile",
        "/change/definition/input",
        "/change/definition/input/selection",
        "/change/definition/input/selection/source",
        "/change/definition/input/selection/source/value",
        "/change/definition/input/qualification",
        "/change/definition/input/qualification/scope_applies",
        "/change/definition/input/qualification/conditions/0",
    ] {
        for array in [false, true] {
            let workflow = Arc::new(Workflow::default());
            let mut value = command();
            let field = value.pointer_mut(pointer).unwrap();
            if array {
                *field = json!([]);
            } else {
                field["ignored"] = json!(true);
            }
            let (status, body) = prepare(workflow.clone(), value).await;
            assert_eq!(status, 400, "{pointer}: {body}");
            assert!(workflow.calls.lock().unwrap().is_empty());
        }
    }
    let workflow = Arc::new(Workflow::default());
    let body = command().to_string().replace(
        "\"expected_revision\":0",
        "\"expected_revision\":0,\"expected_revision\":0",
    );
    let (status, value) = request(
        workflow.clone(),
        "POST",
        &format!("{BASE}/prepare"),
        Some("owner"),
        Some(body),
        &["application/json"],
    )
    .await;
    assert_eq!(status, 400, "{value}");
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn invalid_quantities_revisions_and_repeated_conditions_never_reach_workflow() {
    let mut values = vec![];
    for pointer in [
        "/change/definition/input/ordered_quantity",
        "/change/definition/profile/revision",
        "/change/definition/input/selection/source/value/revision",
    ] {
        let mut value = command();
        *value
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing fixture {pointer}")) = json!(0);
        values.push(value);
    }
    let mut many = command();
    let condition = many["change"]["definition"]["input"]["qualification"]["conditions"][0].clone();
    many["change"]["definition"]["input"]["qualification"]["conditions"] =
        json!(vec![condition; 17]);
    values.push(many);
    let mut duplicate = command();
    let condition =
        duplicate["change"]["definition"]["input"]["qualification"]["conditions"][0].clone();
    duplicate["change"]["definition"]["input"]["qualification"]["conditions"] =
        json!([condition.clone(), condition]);
    values.push(duplicate);
    for value in values {
        let workflow = Arc::new(Workflow::default());
        let (status, body) = prepare(workflow.clone(), value).await;
        assert!(matches!(status, 400 | 422), "{body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
