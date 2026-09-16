#[allow(dead_code)]
mod judicial_calendar_support;
use judicial_calendar_support::*;
use serde_json::json;
use std::sync::Arc;
#[tokio::test]
async fn full_entity_limit_counts_whitespace_and_rejects_the_next_byte() {
    let c = command_json().to_string();
    let exact = format!("{}{}", c, " ".repeat(1024 * 1024 - c.len()));
    let (s, b, calls) = raw(exact.clone()).await;
    assert_eq!(s, 200, "{b}");
    assert_eq!(calls, 1);
    let (s, b, calls) = raw(format!("{exact} ")).await;
    assert_eq!(s, 413);
    assert_eq!(b["error"]["code"], "judicial_calendar_body_too_large");
    assert_eq!(calls, 0);
}
#[tokio::test]
async fn maximum_unicode_literal_and_escaped_commands_are_admitted() {
    let mut c = command_json();
    c["change"] = json!({"action":"replace","expected_revision":u32::MAX-1,"reason":"\u{10000}".repeat(1000),"values":vector("maximum_utf8")["input"]});
    let literal = c.to_string();
    let escaped = literal
        .chars()
        .map(|c| {
            if c.is_ascii() {
                c.to_string()
            } else {
                let mut units = [0u16; 2];
                c.encode_utf16(&mut units)
                    .iter()
                    .map(|u| format!("\\u{u:04x}"))
                    .collect::<String>()
            }
        })
        .collect::<String>();
    assert!(literal.len() > 200_000);
    assert!(escaped.len() > 500_000);
    assert!(escaped.len() < 1024 * 1024);
    for text in [literal, escaped] {
        let envelope = format!(
            "{{\"command\":{text},\"expected_submission_digest\":\"{}\"}}",
            digest().to_hex()
        );
        assert!(envelope.len() < 1024 * 1024);
        let (s, b, calls) = raw(text).await;
        assert_eq!(s, 200, "{b}");
        assert_eq!(calls, 1);
        assert_eq!(b["values"], vector("maximum_utf8")["normalized"]);
        let (s, b) = request(
            Arc::new(Workflow::default()),
            "PUT",
            &format!("{BASE}/{ID}"),
            Some("owner"),
            Some(envelope),
            &["application/json"],
        )
        .await;
        assert_eq!(s, 201, "{b}");
        assert_eq!(b["revision"], u32::MAX);
        assert_eq!(b["values"], vector("maximum_utf8")["normalized"]);
    }
}
#[tokio::test]
async fn content_type_whole_json_and_objects_are_strict() {
    for types in [
        vec![],
        vec!["text/plain"],
        vec!["application/json", "application/json"],
    ] {
        let (s, _) = request(
            Arc::new(Workflow::default()),
            "POST",
            &format!("{BASE}/prepare"),
            Some("owner"),
            Some(command_json().to_string()),
            &types,
        )
        .await;
        assert_eq!(s, 400);
    }
    for text in [
        "[]".into(),
        format!("{} {{}}", command_json()),
        format!("{} null", command_json()),
    ] {
        let (s, _, calls) = raw(text).await;
        assert_eq!(s, 400);
        assert_eq!(calls, 0);
    }
    for pointer in [
        "/change",
        "/change/values",
        "/change/values/scope",
        "/change/values/coverage",
        "/change/values/sources/0",
        "/change/values/weekly_pattern/0",
        "/change/values/exceptions/0",
    ] {
        let mut c = command_json();
        let original = c.pointer(pointer).unwrap().as_object().unwrap();
        let arr = json!(original.values().cloned().collect::<Vec<_>>());
        *c.pointer_mut(pointer).unwrap() = arr;
        let (s, _, calls) = raw(c.to_string()).await;
        assert_eq!(s, 400, "{pointer}");
        assert_eq!(calls, 0);
    }
}
#[tokio::test]
async fn duplicate_unknown_and_foreign_case_fields_are_rejected_everywhere() {
    for pointer in [
        "",
        "/change",
        "/change/values",
        "/change/values/scope",
        "/change/values/coverage",
        "/change/values/sources/0",
        "/change/values/weekly_pattern/0",
        "/change/values/exceptions/0",
    ] {
        let mut c = command_json();
        c.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("case_id".into(), json!(ID));
        let (s, _, calls) = raw(c.to_string()).await;
        assert_eq!(s, 400, "{pointer}");
        assert_eq!(calls, 0);
    }
    let c = command_json().to_string();
    for key in [
        "operation_id",
        "calendar_id",
        "action",
        "expected_revision",
        "title",
        "from",
        "classification",
        "weekday",
        "published_on",
    ] {
        let text = c.replacen(
            &format!("\"{key}\":"),
            &format!("\"{key}\":null,\"{key}\":"),
            1,
        );
        let (s, _, calls) = raw(text).await;
        assert_eq!(s, 400, "{key}");
        assert_eq!(calls, 0);
    }
}
#[tokio::test]
async fn invalid_civil_values_or_revision_do_not_reach_workflow() {
    for (pointer, value) in [
        ("/change/expected_revision", json!(1)),
        ("/change/values/coverage/from", json!("2000-02-30")),
        ("/change/values/scope/entity_codes", json!([" 01"])),
        (
            "/change/values/sources/0/official_url",
            json!("https://user@example.org"),
        ),
        (
            "/change/values/weekly_pattern/0/classification",
            json!("business_day"),
        ),
        ("/change/values/exceptions/0/through", json!("2000-03-30")),
    ] {
        let mut c = command_json();
        *c.pointer_mut(pointer).unwrap() = value;
        let (s, _, calls) = raw(c.to_string()).await;
        assert_eq!(s, 422, "{pointer}");
        assert_eq!(calls, 0);
    }
    for value in [json!(1.0), json!(-1), json!("0"), json!(4294967296u64)] {
        let mut c = command_json();
        c["change"]["expected_revision"] = value;
        let (s, _, calls) = raw(c.to_string()).await;
        assert_eq!(s, 400);
        assert_eq!(calls, 0);
    }
}
