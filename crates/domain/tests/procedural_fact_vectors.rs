#[path = "procedural_fact_support/fixtures.rs"]
mod fixtures;
mod procedural_fact_support;
use procedural_fact_support::{label, text};
use serde_json::Value;

#[test]
fn independent_python_vectors_match_every_canonical_byte_and_direct_reference() {
    let rows: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/procedural_fact_vectors.json")).unwrap();
    assert!(rows.len() >= 12);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        let (bytes, supports) = match row["family"].as_str().unwrap() {
            "resolution" => {
                let input = fixtures::resolution(&row["input"]);
                assert_eq!(input, fixtures::resolution(&row["normalized"]), "{name}");
                (input.canonical_bytes(), input.direct_supports())
            }
            "notification" => {
                let input = fixtures::notification(&row["input"]);
                assert_eq!(input, fixtures::notification(&row["normalized"]), "{name}");
                (input.canonical_bytes(), input.direct_supports())
            }
            _ => panic!("unknown fixture family"),
        };
        let hex: String = bytes.iter().map(|v| format!("{v:02x}")).collect();
        assert_eq!(hex, row["hex"].as_str().unwrap(), "{name}");
        assert_eq!(bytes.len() as u64, row["bytes"].as_u64().unwrap(), "{name}");
        let actual: Vec<_> = supports
            .iter()
            .map(|r| {
                serde_json::json!({
                    "document_id": r.reference().id.to_string(),
                    "version": r.reference().version.get(), "digest": r.digest().to_hex(),
                })
            })
            .collect();
        assert_eq!(Value::Array(actual), row["direct_supports"], "{name}");
    }
}

#[test]
fn canonical_size_bounds_are_attained_by_independent_valid_extreme_values() {
    use domain::procedural_facts::*;
    let rows: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/procedural_fact_vectors.json")).unwrap();
    for (family, minimum, maximum) in [
        (
            "resolution",
            MIN_RESOLUTION_CANONICAL_BYTES,
            MAX_RESOLUTION_CANONICAL_BYTES,
        ),
        (
            "notification",
            MIN_NOTIFICATION_CANONICAL_BYTES,
            MAX_NOTIFICATION_CANONICAL_BYTES,
        ),
    ] {
        let lengths: Vec<usize> = rows
            .iter()
            .filter(|r| r["family"] == family)
            .map(|r| {
                if family == "resolution" {
                    fixtures::resolution(&r["input"]).canonical_bytes().len()
                } else {
                    fixtures::notification(&r["input"]).canonical_bytes().len()
                }
            })
            .collect();
        assert_eq!(lengths.iter().min(), Some(&minimum));
        assert_eq!(lengths.iter().max(), Some(&maximum));
    }
}
