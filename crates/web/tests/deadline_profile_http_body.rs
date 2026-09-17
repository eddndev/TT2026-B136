mod deadline_profile_http_support;
use deadline_profile_http_support::*;
use serde_json::{json, Value};
use std::sync::Arc;

async fn raw(body: String) -> (u16, Value, usize) {
    let w = Arc::new(Workflow::default());
    let (s, b) = request(
        w.clone(),
        "POST",
        &format!("{BASE}/prepare"),
        Some("owner"),
        Some(body),
        &["application/json"],
    )
    .await;
    let calls = w.calls.lock().unwrap().len();
    (s, b, calls)
}
#[tokio::test]
async fn complete_entity_budget_accepts_exact_limit_and_has_profile_specific_413() {
    let text = command_json(None).to_string();
    let exact = format!("{}{}", text, " ".repeat(16 * 1024 * 1024 - text.len()));
    let (s, b, calls) = raw(exact.clone()).await;
    assert_eq!(s, 200, "{b}");
    assert_eq!(calls, 1);
    let (s, b, calls) = raw(format!("{exact} ")).await;
    assert_eq!(s, 413);
    assert_eq!(b["error"]["code"], "deadline_profile_body_too_large");
    assert_eq!(calls, 0);
}
#[tokio::test]
async fn sixteen_maximum_calendar_examples_fit_as_literal_and_ascii_escaped_json() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/judicial_calendar_vectors.json"
    ))
    .unwrap();
    let calendar = vectors
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "maximum_utf8")
        .unwrap()["normalized"]
        .clone();
    let mut c = command_json(None);
    let example = c["change"]["definition"]["examples"][0].clone();
    c["change"]["definition"]["examples"] = json!((0..16)
        .map(|n| {
            let mut e = example.clone();
            e["id"] = json!(uuid::Uuid::from_u128(n + 2));
            e["calendar"] = calendar.clone();
            e
        })
        .collect::<Vec<_>>());
    c["change"]["definition"]["description"] = json!("\u{10000}".repeat(1000));
    let literal = c.to_string();
    let escaped = literal
        .chars()
        .map(|ch| {
            if ch.is_ascii() {
                ch.to_string()
            } else {
                let mut units = [0u16; 2];
                ch.encode_utf16(&mut units)
                    .iter()
                    .map(|u| format!("\\u{u:04x}"))
                    .collect::<String>()
            }
        })
        .collect::<String>();
    assert!(literal.len() > 3_000_000);
    assert!(escaped.len() > 8_000_000);
    assert!(escaped.len() < 16 * 1024 * 1024);
    for text in [literal, escaped] {
        let (s, b, calls) = raw(text).await;
        assert_eq!(s, 200, "{}", b["error"]);
        assert_eq!(calls, 1);
        assert_eq!(b["definition"], c["change"]["definition"]);
    }
}
#[tokio::test]
async fn content_type_trailing_data_and_named_objects_are_strict() {
    for types in [
        vec![],
        vec!["text/plain"],
        vec!["application/json", "application/json"],
    ] {
        let w = Arc::new(Workflow::default());
        let (s, b) = request(
            w.clone(),
            "POST",
            &format!("{BASE}/prepare"),
            Some("owner"),
            Some(command_json(None).to_string()),
            &types,
        )
        .await;
        assert_eq!(s, 400);
        assert_eq!(b["error"]["code"], "invalid_json");
        assert!(w.calls.lock().unwrap().is_empty());
    }
    for text in [
        "[]".into(),
        "null".into(),
        format!("{}{{}}", command_json(None)),
    ] {
        let (s, b, calls) = raw(text).await;
        assert_eq!(s, 400);
        assert_eq!(b["error"]["code"], "invalid_json");
        assert_eq!(calls, 0);
    }
    for pointer in [
        "",
        "/change",
        "/change/definition",
        "/change/definition/scope",
        "/change/definition/scope/value",
        "/change/definition/references/0",
        "/change/definition/trigger",
        "/change/definition/template",
        "/change/definition/template/rule",
        "/change/definition/completion",
        "/change/definition/conditions/0",
        "/change/definition/examples/0",
        "/change/definition/examples/0/anchor",
        "/change/definition/examples/0/expected",
        "/change/definition/examples/0/expected/outcome",
    ] {
        for positional in [false, true] {
            let mut c = command_json(None);
            let value = c.pointer_mut(pointer).unwrap();
            if positional {
                *value = json!([]);
            } else {
                value["unexpected"] = json!(true);
            }
            let (s, b, calls) = raw(c.to_string()).await;
            assert_eq!(s, 400, "{pointer}: {b}");
            assert_eq!(calls, 0);
        }
    }
    for key in [
        "operation_id",
        "action",
        "title",
        "field",
        "quantity",
        "precision",
        "locator",
    ] {
        let c = command_json(None).to_string();
        let marker = format!("\"{key}\":");
        let malformed = c.replacen(&marker, &format!("\"{key}\":null,{marker}"), 1);
        let (s, b, calls) = raw(malformed).await;
        assert_eq!(s, 400, "{key}: {b}");
        assert_eq!(calls, 0);
    }
}
#[tokio::test]
async fn incoherent_examples_and_implicit_temporal_components_are_rejected_before_workflow() {
    for field in [
        "expected",
        "date",
        "offset",
        "quantity",
        "reference",
        "completion",
    ] {
        let mut c = command_json(None);
        let d = &mut c["change"]["definition"];
        match field {
            "expected" => d["examples"][0]["expected"]["outcome"]["date"] = json!("2026-01-07"),
            "date" => d["examples"][0]["anchor"]["day"] = json!(32),
            "offset" => d["examples"][0]["anchor"]["offset_seconds"] = json!(1),
            "quantity" => d["template"]["rule"]["quantity"] = json!(0),
            "reference" => d["conditions"][0]["reference_ids"] = json!([CASE]),
            _ => d["completion"] = json!({"kind":"arithmetic_instant"}),
        }
        let (s, b, calls) = raw(c.to_string()).await;
        assert_eq!(s, 422, "{field}: {b}");
        assert_eq!(calls, 0);
    }
    let mut c = command_json(None);
    c["change"]["definition"]["examples"][0]["anchor"]["hour"] = json!(0);
    let (s, _, calls) = raw(c.to_string()).await;
    assert_eq!(s, 400);
    assert_eq!(calls, 0);
}
