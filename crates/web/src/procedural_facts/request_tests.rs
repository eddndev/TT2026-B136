use super::{request, values};
use application::procedural_facts::ProceduralFactCommand;
use serde_json::{json, Value};

const NIL: &str = "00000000-0000-0000-0000-000000000000";
const OTHER: &str = "00000000-0000-0000-0000-000000000001";

fn vectors() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../domain/tests/fixtures/procedural_fact_vectors.json"
    ))
    .unwrap()
}
fn fixture(name: &str) -> Value {
    vectors().into_iter().find(|v| v["name"] == name).unwrap()
}
fn command(vector: &Value) -> Value {
    let mut result = json!({"family":vector["family"],"operation_id":NIL,"id":NIL,
        "change":{"action":"record","expected_revision":0,"values":vector["input"]}});
    if vector["family"] == "notification" {
        result["resolution_id"] = vector["input"]["resolution"]["id"].clone();
    }
    result
}
fn parsed(value: Value) -> Result<ProceduralFactCommand, crate::error::ApiError> {
    serde_json::from_value::<request::Command>(value)
        .unwrap()
        .validate()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn all_domain_vectors_roundtrip_exact_values_and_normalization() {
    for vector in vectors() {
        let request = command(&vector);
        let model = parsed(request.clone()).unwrap();
        let (bytes, projected) = match &model {
            ProceduralFactCommand::Resolution(v) => {
                let values = v.change().values().unwrap();
                (
                    values.canonical_bytes(),
                    values::resolution(values).unwrap(),
                )
            }
            ProceduralFactCommand::Notification(v) => {
                let values = v.change().values().unwrap();
                (
                    values.canonical_bytes(),
                    values::notification(values).unwrap(),
                )
            }
        };
        assert_eq!(hex(&bytes), vector["hex"], "{}", vector["name"]);
        assert_eq!(projected, vector["normalized"], "{}", vector["name"]);
        let mut expected = request;
        expected["change"]["values"] = vector["normalized"].clone();
        assert_eq!(request::project(&model).unwrap(), expected);
    }
}

#[test]
fn both_families_keep_action_revision_parent_and_normalized_reason() {
    for name in ["resolution_minimum", "notification_minimum"] {
        for action in ["correct", "withdraw"] {
            let mut body = command(&fixture(name));
            body["change"]["action"] = json!(action);
            body["change"]["expected_revision"] = json!(7);
            body["change"]["reason"] = json!("  Reason\r\nSecond line  ");
            if action == "withdraw" {
                body["change"].as_object_mut().unwrap().remove("values");
            }
            let model = parsed(body.clone()).unwrap();
            assert_eq!(model.expected_revision(), 7);
            assert_eq!(model.result_revision().unwrap().get(), 8);
            body["change"]["reason"] = json!("Reason\nSecond line");
            assert_eq!(request::project(&model).unwrap(), body);
        }
    }
}

#[test]
fn invalid_revision_and_parent_are_rejected_without_inference() {
    for (action, revision) in [
        ("record", 1),
        ("correct", 0),
        ("withdraw", 0),
        ("correct", u32::MAX),
        ("withdraw", u32::MAX),
    ] {
        let mut body = command(&fixture("resolution_minimum"));
        body["change"]["action"] = json!(action);
        body["change"]["expected_revision"] = json!(revision);
        if action != "record" {
            body["change"]["reason"] = json!("Correction");
        }
        if action == "withdraw" {
            body["change"].as_object_mut().unwrap().remove("values");
        }
        assert!(parsed(body).is_err());
    }
    let mut body = command(&fixture("notification_minimum"));
    body["resolution_id"] = json!(OTHER);
    assert!(parsed(body).is_err());
}

fn object_paths(value: &Value, path: String, paths: &mut Vec<String>) {
    if let Value::Object(map) = value {
        paths.push(path.clone());
        for (key, child) in map {
            object_paths(child, format!("{path}/{key}"), paths);
        }
    }
}

#[test]
fn all_named_objects_reject_unknown_fields_and_positional_arrays() {
    for name in [
        "resolution_hearing_agreement_support_normalized",
        "notification_shared_support_distinct_locators",
        "notification_explicit_unknown_receipt_effect",
    ] {
        let body = command(&fixture(name));
        let mut paths = Vec::new();
        object_paths(&body, String::new(), &mut paths);
        for path in paths {
            let mut extra = body.clone();
            extra
                .pointer_mut(&path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unexpected".into(), json!(null));
            assert!(
                serde_json::from_value::<request::Command>(extra).is_err(),
                "extra {path}"
            );
            let mut array = body.clone();
            let object = array.pointer_mut(&path).unwrap();
            *object = Value::Array(object.as_object().unwrap().values().cloned().collect());
            assert!(
                serde_json::from_value::<request::Command>(array).is_err(),
                "array {path}"
            );
        }
    }
}

fn duplicate(value: &Value, target: &str, key: &str, path: &str) -> String {
    match value {
        Value::Object(map) => {
            let mut fields = Vec::new();
            for (name, child) in map {
                let item = format!(
                    "{}:{}",
                    serde_json::to_string(name).unwrap(),
                    duplicate(child, target, key, &format!("{path}/{name}"))
                );
                fields.push(item.clone());
                if path == target && name == key {
                    fields.push(item);
                }
            }
            format!("{{{}}}", fields.join(","))
        }
        _ => serde_json::to_string(value).unwrap(),
    }
}

#[test]
fn duplicate_tags_and_fields_are_rejected_at_every_object_depth() {
    for name in [
        "resolution_hearing_agreement_support_normalized",
        "notification_shared_support_distinct_locators",
        "notification_explicit_unknown_receipt_effect",
    ] {
        let body = command(&fixture(name));
        let mut paths = Vec::new();
        object_paths(&body, String::new(), &mut paths);
        for path in paths {
            for key in body.pointer(&path).unwrap().as_object().unwrap().keys() {
                let raw = duplicate(&body, &path, key, "");
                assert!(
                    serde_json::from_str::<request::Command>(&raw).is_err(),
                    "duplicate {path}/{key}"
                );
            }
        }
    }
}

#[test]
fn actions_reject_inapplicable_values_reasons_and_parent_fields() {
    let base = command(&fixture("resolution_minimum"));
    for (path, value) in [
        ("/change/reason", json!("Reason")),
        ("/resolution_id", json!(NIL)),
    ] {
        let mut body = base.clone();
        let (parent, key) = path.rsplit_once('/').unwrap();
        body.pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(key.into(), value);
        assert!(serde_json::from_value::<request::Command>(body).is_err());
    }
    let mut body = base;
    body["change"]["action"] = json!("withdraw");
    body["change"]["expected_revision"] = json!(1);
    body["change"]["reason"] = json!("Reason");
    assert!(serde_json::from_value::<request::Command>(body).is_err());
}

#[test]
fn temporal_precision_rejects_extras_and_accepts_future_declarations() {
    for time in [
        json!({"precision":"unknown","offset_seconds":null}),
        json!({"precision":"date","year":2026,"month":1,"day":1,"hour":0}),
        json!({"precision":"minute","year":2026,"month":1,"day":1,"hour":1,"minute":2,"second":0}),
    ] {
        let mut body = command(&fixture("resolution_minimum"));
        body["change"]["values"]["issued_at"] = time;
        assert!(serde_json::from_value::<request::Command>(body).is_err());
    }
    let mut body = command(&fixture("resolution_year_9999"));
    let model = parsed(body.clone()).unwrap();
    assert_eq!(request::project(&model).unwrap(), body);
    body["change"]["values"]["issued_at"]["offset_seconds"] = json!(-60);
    assert!(parsed(body).is_err());
}

#[test]
fn invalid_time_text_and_support_expectations_use_domain_validation() {
    for time in [
        json!({"precision":"date","year":2026,"month":2,"day":29}),
        json!({"precision":"minute","year":2026,"month":1,"day":1,"hour":24,"minute":0}),
        json!({"precision":"date","year":2026,"month":1,"day":1,"offset_seconds":1}),
    ] {
        let mut body = command(&fixture("resolution_minimum"));
        body["change"]["values"]["issued_at"] = time;
        assert!(parsed(body).is_err());
    }
    for text in ["".to_string(), "x".repeat(1001), "\tx".to_string()] {
        let mut body = command(&fixture("resolution_minimum"));
        body["change"]["values"]["summary"] = json!(text);
        assert!(parsed(body).is_err());
    }
    let mut body = command(&fixture("notification_shared_support_distinct_locators"));
    body["change"]["values"]["representation"]["provenance"]["support"]["digest"] =
        json!("00".repeat(32));
    assert!(parsed(body).is_err());
}

#[test]
fn submission_requires_exact_envelope_and_digest() {
    let body = json!({"command":command(&fixture("notification_minimum")),
        "expected_submission_digest":"ab".repeat(32)});
    let (model, digest) = serde_json::from_value::<request::Submission>(body.clone())
        .unwrap()
        .validate()
        .unwrap();
    assert_eq!(digest.to_hex(), "ab".repeat(32));
    assert_eq!(request::project(&model).unwrap(), body["command"]);
    for value in [
        json!([]),
        json!({"command":body["command"]}),
        json!({"command":body["command"],"expected_submission_digest":"00".repeat(32),"extra":null}),
    ] {
        assert!(serde_json::from_value::<request::Submission>(value).is_err());
    }
    let mut bad = body.clone();
    bad["expected_submission_digest"] = json!("xyz");
    assert!(serde_json::from_value::<request::Submission>(bad)
        .unwrap()
        .validate()
        .is_err());
    let raw = duplicate(&body, "", "command", "");
    assert!(serde_json::from_str::<request::Submission>(&raw).is_err());
}

#[test]
fn omission_only_applies_to_optional_fields_not_unknown_declarations() {
    let mut body = command(&fixture("notification_shared_support_distinct_locators"));
    for pointer in [
        "/change/values/subtype",
        "/change/values/received_at",
        "/change/values/stated_effect",
        "/change/values/provenance/support",
        "/change/values/representation/provenance/support",
        "/change/values/practiced_at/offset_seconds",
    ] {
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        body.pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
    }
    let projected = request::project(&parsed(body).unwrap()).unwrap();
    for pointer in [
        "/change/values/subtype",
        "/change/values/received_at",
        "/change/values/stated_effect",
        "/change/values/provenance/support",
        "/change/values/representation/provenance/support",
        "/change/values/practiced_at/offset_seconds",
    ] {
        assert_eq!(projected.pointer(pointer), Some(&Value::Null), "{pointer}");
    }
    for field in ["class", "issuer", "issued_at", "provenance"] {
        for null in [false, true] {
            let mut body = command(&fixture("resolution_minimum"));
            let values = body["change"]["values"].as_object_mut().unwrap();
            if null {
                values.insert(field.into(), Value::Null);
            } else {
                values.remove(field);
            }
            assert!(
                serde_json::from_value::<request::Command>(body).is_err(),
                "{field}"
            );
        }
    }
}
