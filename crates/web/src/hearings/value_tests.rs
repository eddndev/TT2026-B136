use super::{request::Command, time::parse_time, values::Values};
use serde_json::{json, Value};

pub(super) fn values() -> Value {
    json!({"kind":"initial","scheduled_at":"2026-10-01T09:00:15-06:00", "modality":"in_person", "venue":" Room ", "note":null, "participants":[], "conviction_basis":null})
}
pub(super) fn command() -> Value {
    json!({"operation_id":"00000000-0000-4000-8000-000000000011", "hearing_id":"00000000-0000-4000-8000-000000000012", "change":{"action":"schedule","expected_revision":0,"expected_case_revision":1,"expected_stage_revision":1,"values":values()}})
}
#[test]
fn valid_time_retains_seconds_offset_and_future_date() {
    let at = parse_time("2026-10-01T09:00:15-06:00").unwrap();
    assert_eq!(at.value().second(), 15);
    assert_eq!(at.value().offset().whole_seconds(), -21600);
    assert!(parse_time("0001-01-01T00:00:00Z").is_ok());
    assert!(parse_time("9999-12-31T23:59:59Z").is_ok());
    assert!(parse_time("2024-02-29T23:59:59+14:00").is_ok());
}
#[test]
fn time_rejects_unknown_offset_fraction_leap_seconds_and_calendar_overflow() {
    for bad in [
        "2026-01-01T09:00:00-00:00",
        "2026-01-01T09:00:00.1Z",
        "2026-01-01T09:00:60Z",
        "2026-01-01T09:00:00+14:01",
        "2026-02-29T09:00:00Z",
        "2026-04-31T09:00:00Z",
        "0000-01-01T00:00:00Z",
        "0001-01-01T00:00:00+00:01",
        "9999-12-31T23:59:59-00:01",
        "2026-01-01 09:00:00Z",
        "2026-01-01T09:00:00",
        "2026-01-01T09:00:00Z\n",
    ] {
        assert!(parse_time(bad).is_err(), "accepted {bad:?}");
    }
}
#[test]
fn command_preserves_explicit_context_and_normalizes_values() {
    let parsed = serde_json::from_value::<Command>(command())
        .unwrap()
        .validate()
        .unwrap();
    match parsed.change {
        application::hearings::HearingChange::Schedule { context, values } => {
            assert_eq!(context.case_revision.get(), 1);
            assert_eq!(context.stage_revision.get(), 1);
            assert_eq!(values.venue().as_str(), "Room");
        }
        _ => panic!("wrong command variant"),
    }
}
#[test]
fn syntax_rejects_unknown_duplicate_and_cross_variant_fields() {
    let raw = serde_json::to_string(&command()).unwrap();
    let duplicate = raw.replacen(
        "\"operation_id\":",
        "\"operation_id\":\"00000000-0000-4000-8000-000000000011\",\"operation_id\":",
        1,
    );
    assert!(serde_json::from_str::<Command>(&duplicate).is_err());
    let mut unknown = command();
    unknown["extra"] = json!(1);
    assert!(serde_json::from_value::<Command>(unknown).is_err());
    let mut cancel = command();
    cancel["change"] =
        json!({"action":"cancel","expected_revision":1,"reason":"Cancelled","values":values()});
    assert!(serde_json::from_value::<Command>(cancel).is_err());
}
#[test]
fn schedule_cannot_claim_existing_revision_or_unknown_context() {
    for (field, bad) in [
        ("expected_revision", 1),
        ("expected_case_revision", 0),
        ("expected_stage_revision", 0),
    ] {
        let mut value = command();
        value["change"][field] = json!(bad);
        assert!(serde_json::from_value::<Command>(value)
            .unwrap()
            .validate()
            .is_err());
    }
}
#[test]
fn sentencing_requires_only_its_exact_support_and_statement() {
    let mut value = values();
    value["kind"] = json!("sentencing");
    assert!(serde_json::from_value::<Values>(value.clone())
        .unwrap()
        .validate()
        .is_err());
    value["conviction_basis"] = json!({"statement":"Declared finding","support":{"document_id":"00000000-0000-4000-8000-000000000013","version":1,"digest":"ab".repeat(32)}});
    assert!(serde_json::from_value::<Values>(value.clone())
        .unwrap()
        .validate()
        .is_ok());
    value["conviction_basis"]["support"]["version"] = json!(0);
    assert!(serde_json::from_value::<Values>(value)
        .unwrap()
        .validate()
        .is_err());
}
