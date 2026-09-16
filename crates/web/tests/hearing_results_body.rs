#[allow(dead_code)]
mod hearing_result_support;
use hearing_result_support::*;
use serde_json::{json, Value};
use std::sync::Arc;
async fn raw(text: String, types: &[&str]) -> (u16, Value, usize) {
    let w = Arc::new(Workflow::default());
    let (status, body) = request(
        w.clone(),
        "POST",
        &format!("{}/prepare", base_url()),
        Some("valid"),
        Some(text),
        types,
    )
    .await;
    let calls = w.calls.lock().unwrap().len();
    (status, body, calls)
}
#[tokio::test]
async fn requires_one_json_content_type_named_objects_and_complete_input() {
    for headers in [
        vec![],
        vec!["text/plain"],
        vec!["application/json", "application/json"],
    ] {
        let (status, _, calls) = raw(command_json().to_string(), &headers).await;
        assert_eq!(status, 400);
        assert_eq!(calls, 0);
    }
    for suffix in ["{}", "null", " trailing"] {
        let (status, _, calls) =
            raw(format!("{}{suffix}", command_json()), &["application/json"]).await;
        assert_eq!(status, 400);
        assert_eq!(calls, 0);
    }
    let c = command_json();
    let mut bodies = vec![json!([
        c["operation_id"],
        c["hearing_id"],
        c["result_id"],
        c["change"]
    ])];
    for field in ["event_time", "provenance", "attendees", "agreements"] {
        let mut c = c.clone();
        c["change"]["values"][field] = if field == "attendees" || field == "agreements" {
            json!([["x", 1]])
        } else {
            json!(["x"])
        };
        bodies.push(c);
    }
    for body in bodies {
        let (status, _, calls) = raw(body.to_string(), &["application/json"]).await;
        assert_eq!(status, 400);
        assert_eq!(calls, 0);
    }
}
#[tokio::test]
async fn unknown_and_duplicate_fields_are_rejected_at_each_nested_boundary() {
    let original = command_json().to_string();
    for text in [
        original.replace("\"summary\":", "\"summary\":\"first\",\"summary\":"),
        original.replace("\"precision\":", "\"precision\":\"date\",\"precision\":"),
        original.replace("\"kind\":", "\"kind\":\"operator_note\",\"kind\":"),
        original.replace("\"action\":", "\"action\":\"record\",\"action\":"),
    ] {
        let (status, _, calls) = raw(text, &["application/json"]).await;
        assert_eq!(status, 400);
        assert_eq!(calls, 0);
    }
    for pointer in [
        "",
        "/change",
        "/change/values",
        "/change/values/event_time",
        "/change/values/provenance",
    ] {
        let mut c = command_json();
        c.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!(true));
        let (status, _, calls) = raw(c.to_string(), &["application/json"]).await;
        assert_eq!(status, 400);
        assert_eq!(calls, 0);
    }
}
#[tokio::test]
async fn byte_limit_includes_whitespace_and_admits_more_than_scheduling_limit() {
    let c = command_json().to_string();
    let padded = format!("{}{}", c, " ".repeat(512 * 1024 - c.len()));
    let (status, body, calls) = raw(padded.clone(), &["application/json"]).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(calls, 1);
    let (status, body, calls) = raw(format!("{padded} "), &["application/json"]).await;
    assert_eq!(status, 413);
    assert_eq!(body["error"]["code"], "hearing_result_body_too_large");
    assert_eq!(calls, 0);
}
#[tokio::test]
async fn maximum_unicode_values_pass_the_http_limit_and_preserve_agreement_order() {
    let text = "\u{10000}".repeat(1000);
    let mut c = command_json();
    c["change"] =
        json!({"action":"correct","expected_revision":1,"reason":text,"values":values_json()});
    let v = &mut c["change"]["values"];
    v["summary"] = json!(text);
    v["event_time"] = json!({"precision":"instant","at":"2026-09-16T00:00:00-06:00"});
    v["attendees"]=json!((1..=32).rev().map(|id|json!({"participant_id":uuid::Uuid::from_u128(id).to_string(),"revision":u32::MAX,"capacity":"\u{10000}".repeat(100),"observation":"\u{10000}".repeat(500)})).collect::<Vec<_>>());
    v["agreements"] = json!((1..=16)
        .rev()
        .map(|id| json!({"id":uuid::Uuid::from_u128(id).to_string(),"text":text}))
        .collect::<Vec<_>>());
    v["provenance"] = json!({"kind":"written_record","reference":"\u{10000}".repeat(200),"support":{"document_id":HEARING,"version":u32::MAX,"digest":digest().to_hex()}});
    let text = c.to_string();
    assert!(text.len() > 64 * 1024 && text.len() < 512 * 1024);
    let (status, body, calls) = raw(text, &["application/json"]).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(calls, 1);
    assert_eq!(
        body["values"]["attendees"][0]["participant_id"],
        uuid::Uuid::from_u128(1).to_string()
    );
    assert_eq!(
        body["values"]["agreements"][0]["id"],
        uuid::Uuid::from_u128(16).to_string()
    );
}
#[tokio::test]
async fn invalid_event_precision_offsets_and_values_never_reach_application() {
    let invalid = vec![
        json!({"precision":"date","date":"2026-02-30","offset":"-06:00"}),
        json!({"precision":"date","date":"2026-9-16","offset":"-06:00"}),
        json!({"precision":"date","date":"2026-09-16","offset":"-00:00"}),
        json!({"precision":"date","date":"2026-09-16","offset":"+14:01"}),
        json!({"precision":"date","date":"0001-01-01","offset":"+14:00"}),
    ];
    for at in [
        "2026-09-16t10:00:00Z",
        "2026-09-16T10:00:00z",
        "2026-09-16T10:00:00-00:00",
        "2026-09-16T10:00:60Z",
        "2026-09-16T10:00:00.1Z",
        "2026-09-16T10:00:00",
    ] {
        let mut c = command_json();
        c["change"]["values"]["event_time"] = json!({"precision":"instant","at":at});
        let (status, _, calls) = raw(c.to_string(), &["application/json"]).await;
        assert_eq!(status, 422, "{at}");
        assert_eq!(calls, 0);
    }
    for time in invalid {
        let mut c = command_json();
        c["change"]["values"]["event_time"] = time;
        let (status, _, calls) = raw(c.to_string(), &["application/json"]).await;
        assert_eq!(status, 422);
        assert_eq!(calls, 0);
    }
    for (key, value) in [
        ("occurrence", json!("invented")),
        ("summary", json!(" ")),
        ("summary", json!("bad\u{7f}")),
        (
            "attendees",
            json!([{"participant_id":RESULT,"revision":0,"capacity":"role"}]),
        ),
        (
            "agreements",
            json!((0..17)
                .map(|_| json!({"id":RESULT,"text":"declared"}))
                .collect::<Vec<_>>()),
        ),
    ] {
        let mut c = command_json();
        c["change"]["values"][key] = value;
        let (status, _, calls) = raw(c.to_string(), &["application/json"]).await;
        assert_eq!(status, 422);
        assert_eq!(calls, 0);
    }
}
