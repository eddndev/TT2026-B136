mod case_administration_support;
use case_administration_support::Fixture;
use serde_json::{json, Value};

fn common() -> Value {
    json!({"change_kind":"to_intermediate","from_stage":"investigation","stage":"intermediate",
        "act_precision":"instant","act_seconds":1789441200i64,"act_nanoseconds":123456789,"act_offset_seconds":-21600,
        "support_id":"00112233-4455-6677-8899-aabbccddeeff","support_version":7,
        "support_digest":"\\x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        "support_name":"support.pdf","support_format":"pdf","support_policy":"pdf_docx_v1"})
}
fn trial() -> Value {
    let mut value = common();
    value["change_kind"] = "to_trial".into();
    value["from_stage"] = "intermediate".into();
    value["stage"] = "trial".into();
    value["act_precision"] = "date".into();
    value["act_date"] = "2026-09-14".into();
    value["act_seconds"] = Value::Null;
    value["act_nanoseconds"] = Value::Null;
    value["received_precision"] = "instant".into();
    value["received_seconds"] = 1789410601i64.into();
    value["received_nanoseconds"] = 1.into();
    value["received_offset_seconds"] = 19800.into();
    value["receiving_court"] = "Juzgado \u{e1}".into();
    value["receipt_reference"] = "R-1".into();
    value["note"] = "\u{e1}\nb".into();
    for field in ["id", "version", "digest", "name", "format", "policy"] {
        value[format!("receipt_{field}")] = value[format!("support_{field}")].clone();
    }
    value
}
fn hash(db: &mut Fixture, value: &Value) -> (i32, String) {
    let row=db.admin.query_one("SELECT octet_length(case_stage_values_bytes(v)),encode(sha256(case_stage_values_bytes(v)),'hex') FROM jsonb_populate_record(NULL::case_stage_revisions,$1) v",&[value]).unwrap();
    (row.get(0), row.get(1))
}
fn valid(db: &mut Fixture, value: &Value) -> bool {
    db.admin.query_one("SELECT case_stage_values_canonical(jsonb_populate_record(NULL::case_stage_revisions,$1))",&[value]).unwrap().get(0)
}
#[test]
fn instant_and_mixed_precision_vectors_match_independent_exact_hashes() {
    let Some(mut db) = Fixture::new() else { return };
    assert_eq!(
        hash(&mut db, &common()),
        (
            76,
            "967505133342841db8c6f4e3b807d4ee389a50d977edfca096e779903b885177".into()
        )
    );
    assert_eq!(
        hash(&mut db, &trial()),
        (
            168,
            "b730b5ba7203a2aa20477e3f57bc9ce74d87f382777cfdbcfc5968387798beab".into()
        )
    );
}
#[test]
fn maximum_unicode_texts_fit_exact_canonical_bound_and_one_more_scalar_is_rejected() {
    let Some(mut db) = Fixture::new() else { return };
    let mut value = trial();
    value["act_precision"] = "instant".into();
    value["act_date"] = Value::Null;
    value["act_seconds"] = 1789410601i64.into();
    value["act_nanoseconds"] = 1.into();
    value["receiving_court"] = "\u{1f600}".repeat(200).into();
    value["receipt_reference"] = "\u{1f600}".repeat(200).into();
    value["note"] = "\u{1f600}".repeat(1000).into();
    assert_eq!(hash(&mut db, &value).0, 5759);
    value["note"] = "\u{1f600}".repeat(1001).into();
    assert!(!valid(&mut db, &value));
}
#[test]
fn sql_rejects_noncanonical_text_partial_receipts_and_conflicting_temporal_intervals() {
    let Some(mut db) = Fixture::new() else { return };
    for (field, value) in [
        ("receiving_court", json!(" Court")),
        ("note", json!("First\r\nSecond")),
        ("note", json!("First\tSecond")),
        ("receipt_reference", json!("")),
        ("receipt_policy", Value::Null),
        ("received_seconds", json!(1789320000i64)),
        ("act_offset_seconds", json!(61)),
    ] {
        let mut row = trial();
        row[field] = value;
        assert!(!valid(&mut db, &row), "{field}");
    }
}
#[test]
fn same_day_unknown_precision_allows_overlap_but_recording_cannot_precede_day_start() {
    let Some(mut db) = Fixture::new() else { return };
    let mut value = trial();
    value["received_precision"] = "date".into();
    value["received_date"] = "2026-09-14".into();
    value["received_seconds"] = Value::Null;
    value["received_nanoseconds"] = Value::Null;
    value["received_offset_seconds"] = (-21600).into();
    value["recorded_at_seconds"] = 1789365600i64.into();
    value["recorded_at_nanoseconds"] = 0.into();
    assert!(valid(&mut db, &value));
    let check = |db: &mut Fixture, row: &Value| -> bool {
        db.admin.query_one("SELECT case_stage_recording_valid(jsonb_populate_record(NULL::case_stage_revisions,$1))",&[row]).unwrap().get(0)
    };
    assert!(check(&mut db, &value));
    value["recorded_at_seconds"] = 1789365599i64.into();
    value["recorded_at_nanoseconds"] = 999999999.into();
    assert!(!check(&mut db, &value));
}
