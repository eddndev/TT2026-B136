mod procedural_fact_http_support;
use procedural_fact_http_support::*;
use serde_json::{json, Value};
use std::sync::Arc;

#[tokio::test]
async fn every_independent_value_vector_reaches_application_without_semantic_changes() {
    let rows = vectors();
    assert_eq!(rows.len(), 28);
    for vector in rows {
        let family = vector["family"].as_str().unwrap();
        let mut command = command_json(family, "record");
        command["change"]["values"] = vector["input"].clone();
        let mut path = base_url(family);
        if family == "notification" {
            let parent = command["change"]["values"]["resolution"]["id"].clone();
            command["resolution_id"] = parent.clone();
            path = path.replace(PARENT, parent.as_str().unwrap());
        }
        let workflow = Arc::new(Workflow::default());
        let (_, body) = request(
            workflow.clone(),
            "POST",
            &format!("{path}/prepare"),
            Some("capture"),
            Some(command.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(
            workflow.calls.lock().unwrap().len(),
            1,
            "{}: {body}",
            vector["name"]
        );
        let mut normalized = command.clone();
        normalized["change"]["values"] = vector["normalized"].clone();
        assert_eq!(
            workflow.commands.lock().unwrap().as_slice(),
            &[typed_command(&normalized)],
            "{}",
            vector["name"]
        );
    }
}

#[tokio::test]
async fn maximum_unicode_values_also_fit_when_escaped_as_ascii_json() {
    for vector in vectors()
        .into_iter()
        .filter(|v| v["name"].as_str().unwrap().ends_with("maximum_utf8"))
    {
        let family = vector["family"].as_str().unwrap();
        let mut command = command_json(family, "correct");
        command["change"]["values"] = vector["normalized"].clone();
        command["change"]["reason"] = json!("\u{10000}".repeat(1000));
        let mut path = base_url(family);
        if family == "notification" {
            let parent = command["change"]["values"]["resolution"]["id"].clone();
            command["resolution_id"] = parent.clone();
            path = path.replace(PARENT, parent.as_str().unwrap());
        }
        let body = submission(command.clone())
            .to_string()
            .chars()
            .map(|ch| {
                if ch.is_ascii() {
                    ch.to_string()
                } else {
                    let mut units = [0_u16; 2];
                    ch.encode_utf16(&mut units)
                        .iter()
                        .map(|u| format!("\\u{u:04x}"))
                        .collect::<String>()
                }
            })
            .collect::<String>();
        assert!(body.is_ascii());
        assert!(body.len() < 512 * 1024);
        let workflow = Arc::new(Workflow::default());
        let (_, response) = request(
            workflow.clone(),
            "PUT",
            &format!("{path}/{ID}"),
            Some("capture"),
            Some(body),
            &["application/json"],
        )
        .await;
        assert_eq!(workflow.calls.lock().unwrap().len(), 1, "{response}");
        assert_eq!(
            workflow.commands.lock().unwrap().as_slice(),
            &[typed_command(&command)]
        );
    }
}

#[tokio::test]
async fn temporal_range_and_precision_are_validated_without_inventing_components() {
    for value in [
        json!({"precision":"date","year":0,"month":1,"day":1,"offset_seconds":null}),
        json!({"precision":"date","year":10000,"month":1,"day":1,"offset_seconds":null}),
        json!({"precision":"date","year":2026,"month":2,"day":29,"offset_seconds":null}),
        json!({"precision":"date","year":2026,"month":4,"day":31,"offset_seconds":null}),
        json!({"precision":"date","year":2026,"month":9,"day":16,"offset_seconds":61}),
        json!({"precision":"date","year":2026,"month":9,"day":16,"offset_seconds":50460}),
        json!({"precision":"date","year":1,"month":1,"day":1,"offset_seconds":50400}),
        json!({"precision":"minute","year":2026,"month":9,"day":16,"hour":24,"minute":0,"offset_seconds":null}),
        json!({"precision":"second","year":2026,"month":9,"day":16,"hour":1,"minute":0,"second":60,"offset_seconds":0}),
    ] {
        rejects_value("resolution", "/issued_at", value, 422).await;
    }
    for value in [
        json!({"precision":"unknown","offset_seconds":0}),
        json!({"precision":"date","year":2026,"month":9,"day":16,"hour":0,"offset_seconds":null}),
        json!({"precision":"minute","year":2026,"month":9,"day":16,"hour":0,"minute":0,"second":0,"offset_seconds":null}),
        json!({"precision":"second","year":2026,"month":9,"day":16,"hour":0,"minute":0,"second":0.5,"offset_seconds":null}),
    ] {
        rejects_value("resolution", "/issued_at", value, 400).await;
    }
}

#[tokio::test]
async fn invalid_text_catalogs_people_and_document_references_never_reach_workflow() {
    for (family, path, value) in [
        ("resolution", "/summary", json!(" ")),
        ("resolution", "/summary", json!("x".repeat(1001))),
        ("resolution", "/summary", json!("bad\u{7f}")),
        (
            "resolution",
            "/issuer",
            json!({"kind":"known","value":"x".repeat(201)}),
        ),
        (
            "resolution",
            "/class",
            json!({"kind":"unknown","reason":""}),
        ),
        ("notification", "/resolution/revision", json!(0)),
        (
            "notification",
            "/intended_recipient",
            json!({"kind":"known","value":{"kind":"participant","id":ID,"revision":0}}),
        ),
        (
            "notification",
            "/actual_receiver",
            json!({"kind":"known","value":{"kind":"unlinked","label":"x","description":""}}),
        ),
        (
            "notification",
            "/representation",
            json!({"kind":"not_recorded","reason":""}),
        ),
        (
            "notification",
            "/stated_effect",
            json!({"at":{"precision":"unknown"},"statement":"","locator":"x"}),
        ),
        (
            "resolution",
            "/provenance",
            json!({"kind":"hearing_result","reference":{"hearing_id":ID,"result_id":OPERATION,"revision":0,"agreement_id":null},"locator":"x","support":null}),
        ),
    ] {
        rejects_value(family, path, value, 422).await;
    }
    rejects_value(
        "resolution",
        "/provenance",
        json!({"kind":"external_reference","reference":"x","support":{"document_id":ID,"version":0,"digest":digest().to_hex(),"locator":"x"}}),
        400,
    ).await;
}

#[tokio::test]
async fn malformed_uuid_digest_and_integer_types_are_not_silently_coerced() {
    for (path, value) in [
        ("/operation_id", json!("not-an-id")),
        ("/id", json!("not-an-id")),
        ("/change/expected_revision", json!("0")),
        ("/change/expected_revision", json!(0.5)),
        ("/change/expected_revision", json!(-1)),
        ("/change/expected_revision", json!(4294967296_u64)),
    ] {
        let mut command = command_json("resolution", "record");
        *command.pointer_mut(path).unwrap() = value;
        rejects_command("resolution", command, 400).await;
    }
    for digest in [
        "",
        "aa",
        "gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg",
    ] {
        let mut payload = submission(command_json("resolution", "record"));
        payload["expected_submission_digest"] = json!(digest);
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            "POST",
            &base_url("resolution"),
            Some("valid"),
            Some(payload.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert_eq!(body["error"]["code"], "invalid_procedural_fact_digest");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn action_revisions_and_required_reasons_are_validated_before_workflow() {
    for family in ["resolution", "notification"] {
        let mut bad_record = command_json(family, "record");
        bad_record["change"]["expected_revision"] = json!(1);
        rejects_command(family, bad_record, 422).await;
        for action in ["correct", "withdraw"] {
            let mut zero = command_json(family, action);
            zero["change"]["expected_revision"] = json!(0);
            rejects_command(family, zero, 422).await;
            let mut blank = command_json(family, action);
            blank["change"]["reason"] = json!(" ");
            rejects_command(family, blank, 422).await;
            let mut missing = command_json(family, action);
            missing["change"].as_object_mut().unwrap().remove("reason");
            rejects_command(family, missing, 400).await;
        }
    }
}

async fn rejects_value(family: &str, path: &str, value: Value, expected: u16) {
    let mut command = command_json(family, "record");
    *command["change"]["values"].pointer_mut(path).unwrap() = value;
    rejects_command(family, command, expected).await;
}
async fn rejects_command(family: &str, command: Value, expected: u16) {
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        workflow.clone(),
        "POST",
        &format!("{}/prepare", base_url(family)),
        Some("valid"),
        Some(command.to_string()),
        &["application/json"],
    )
    .await;
    assert_eq!(status, expected, "{body}");
    assert!(workflow.calls.lock().unwrap().is_empty());
}
