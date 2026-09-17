mod case_administration_support;

use case_administration_support::Fixture;
use serde_json::Value;

fn bytes(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|part| u8::from_str_radix(std::str::from_utf8(part).unwrap(), 16).unwrap())
        .collect()
}
fn vectors() -> Value {
    serde_json::from_str(include_str!("fixtures/hearing_result_receipts.json")).unwrap()
}

#[test]
fn sql_decodes_independent_receipt_vectors_at_minimum_and_maximum_sizes() {
    let Some(mut db) = Fixture::new() else { return };
    for vector in vectors().as_array().unwrap() {
        let canonical = bytes(vector["hex"].as_str().unwrap());
        let row = db
            .admin
            .query_one(
                "SELECT hearing_result_submission($1),encode(sha256($1),'hex')",
                &[&canonical],
            )
            .unwrap();
        assert_eq!(row.get::<_, Value>(0), vector["projection"]);
        assert_eq!(row.get::<_, String>(1), vector["sha256"]);
        assert_eq!(canonical.len() as u64, vector["bytes"]);
    }
}

#[test]
fn sql_rejects_truncated_and_inconsistent_result_receipts() {
    let Some(mut db) = Fixture::new() else { return };
    let original = bytes(vectors()[0]["hex"].as_str().unwrap());
    let mut invalid: Vec<Vec<u8>> = (0..original.len())
        .map(|n| original[..n].to_vec())
        .collect();
    for (index, byte) in [(0, b'X'), (85, 3), (89, 1), (93, 0), (158, 2), (191, 1)] {
        let mut value = original.clone();
        value[index] = byte;
        invalid.push(value);
    }
    let mut exhausted = original.clone();
    exhausted[86..90].copy_from_slice(&u32::MAX.to_be_bytes());
    invalid.push(exhausted);
    let mut trailing = original;
    trailing.push(0);
    invalid.push(trailing);
    for canonical in invalid {
        let error = db
            .admin
            .query_one("SELECT hearing_result_submission($1)", &[&canonical])
            .unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}
