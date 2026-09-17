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

#[test]
fn sql_decodes_independent_result_vectors_at_both_size_limits() {
    let Some(mut db) = Fixture::new() else { return };
    let vectors: Value = serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/hearing_result_vectors.json"
    ))
    .unwrap();
    let mut sizes = Vec::new();
    for vector in vectors.as_array().unwrap() {
        let canonical = bytes(vector["hex"].as_str().unwrap());
        let row = db
            .admin
            .query_one(
                "SELECT hearing_result_values($1),encode(sha256($1),'hex')",
                &[&canonical],
            )
            .unwrap();
        let value: Value = row.get(0);
        assert_eq!(row.get::<_, String>(1), vector["sha256"]);
        for key in [
            "occurrence",
            "extent",
            "summary",
            "agreements",
            "provenance",
        ] {
            assert_eq!(
                value[key], vector["input"][key],
                "{}: {key}",
                vector["name"]
            );
        }
        let mut expected = vector["input"]["attendees"].as_array().unwrap().clone();
        expected.sort_by_key(|v| v["participant_id"].as_str().unwrap().to_owned());
        assert_eq!(value["attendees"], serde_json::json!(expected));
        assert_eq!(
            value["event_time"]["precision"],
            vector["input"]["event_time"]["precision"]
        );
        sizes.push(canonical.len());
    }
    assert_eq!(*sizes.iter().min().unwrap(), 26);
    assert_eq!(*sizes.iter().max().unwrap(), 146_933);
}

#[test]
fn sql_rejects_truncation_tags_noncanonical_text_and_cross_year_dates() {
    let Some(mut db) = Fixture::new() else { return };
    let original = bytes("4852455331000200000101010000000000000001780000000000");
    let mut invalid: Vec<Vec<u8>> = (0..original.len())
        .map(|n| original[..n].to_vec())
        .collect();
    for (index, byte) in [
        (0, b'X'),
        (5, 2),
        (6, 3),
        (7, 2),
        (8, 0xff),
        (10, 13),
        (11, 0),
        (20, b' '),
        (21, 33),
        (22, 17),
        (23, 3),
        (24, 2),
        (25, 2),
    ] {
        let mut value = original.clone();
        value[index] = byte;
        invalid.push(value);
    }
    let mut trailing = original.clone();
    trailing.push(0);
    invalid.push(trailing);
    let mut lower = original.clone();
    lower[12..16].copy_from_slice(&50_400_i32.to_be_bytes());
    invalid.push(lower);
    let mut upper = original.clone();
    upper[8..10].copy_from_slice(&9999_u16.to_be_bytes());
    upper[10] = 12;
    upper[11] = 31;
    upper[12..16].copy_from_slice(&(-50_400_i32).to_be_bytes());
    invalid.push(upper);
    for canonical in invalid {
        let error = db
            .admin
            .query_one("SELECT hearing_result_values($1)", &[&canonical])
            .unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}

#[test]
fn canonical_date_projection_does_not_depend_on_postgresql_date_style() {
    let Some(mut db) = Fixture::new() else { return };
    let vectors: Value = serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/hearing_result_vectors.json"
    ))
    .unwrap();
    for style in ["ISO, MDY", "SQL, DMY", "German, DMY", "Postgres, MDY"] {
        db.admin
            .execute("SELECT set_config('DateStyle',$1,FALSE)", &[&style])
            .unwrap();
        for vector in vectors
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["input"]["event_time"]["precision"] == "date")
        {
            let canonical = bytes(vector["hex"].as_str().unwrap());
            let projected: Value = db
                .admin
                .query_one("SELECT hearing_result_values($1)", &[&canonical])
                .unwrap()
                .get(0);
            assert_eq!(
                projected["event_time"]["date"], vector["input"]["event_time"]["date"],
                "{style}"
            );
            let decoded =
                infrastructure::hearing_result_codec::values(&canonical, &projected).unwrap();
            assert_eq!(decoded.canonical_bytes(), canonical);
        }
    }
}
