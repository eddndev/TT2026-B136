mod case_administration_support;

use case_administration_support::Fixture;
use infrastructure::hearing_result_codec::values;
use serde_json::{json, Value};

fn bytes(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn strict_result_codec_rebuilds_all_independent_values_without_normalization() {
    let Some(mut db) = Fixture::new() else { return };
    let vectors: Value = serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/hearing_result_vectors.json"
    ))
    .unwrap();
    for vector in vectors.as_array().unwrap() {
        let canonical = bytes(vector["hex"].as_str().unwrap());
        let projection: Value = db
            .admin
            .query_one("SELECT hearing_result_values($1)", &[&canonical])
            .unwrap()
            .get(0);
        let result = values(&canonical, &projection).unwrap();
        assert_eq!(result.canonical_bytes(), canonical);
        assert_eq!(
            result.summary().as_str(),
            vector["input"]["summary"].as_str().unwrap()
        );
    }
}
#[test]
fn strict_result_codec_rejects_damaged_or_noncanonical_projections() {
    let Some(mut db) = Fixture::new() else { return };
    let canonical = bytes("4852455331000200000101010000000000000001780000000000");
    let base: Value = db
        .admin
        .query_one("SELECT hearing_result_values($1)", &[&canonical])
        .unwrap()
        .get(0);
    assert!(values(&canonical, &base).is_ok());
    for (field, value) in [
        ("summary", json!(" x ")),
        ("summary", json!("")),
        ("summary", json!(42)),
        ("occurrence", json!("unknown")),
        ("extent", json!("partial")),
        ("attendees", json!(null)),
        ("agreements", json!({})),
        ("provenance", json!({"kind":"operator_note"})),
        (
            "event_time",
            json!({"precision":"date","date":"0001-01-01","offset_seconds":60}),
        ),
        ("extra", json!(1)),
    ] {
        let mut changed = base.clone();
        changed[field] = value;
        assert!(values(&canonical, &changed).is_err(), "{field}");
    }
    let mut truncated = canonical.clone();
    truncated.pop();
    assert!(values(&truncated, &base).is_err());
}
