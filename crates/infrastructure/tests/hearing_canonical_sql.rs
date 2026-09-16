mod case_administration_support;

use case_administration_support::Fixture;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn vectors() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/hearing_vectors.json"
    ))
    .unwrap()
}

#[test]
fn sql_decodes_independent_hearing_vectors_with_exact_offsets_and_sorted_references() {
    let Some(mut db) = Fixture::new() else { return };
    for vector in vectors() {
        let mut expected = vector["input"].clone();
        let at = OffsetDateTime::parse(expected["time"].as_str().unwrap(), &Rfc3339).unwrap();
        expected["time"] = serde_json::json!({
            "seconds":at.unix_timestamp(),"offset_seconds":at.offset().whole_seconds()
        });
        expected["participants"]
            .as_array_mut()
            .unwrap()
            .sort_by_key(|v| v["id"].as_str().unwrap().to_owned());
        let row = db
            .admin
            .query_one(
                "SELECT hearing_values(decode($1,'hex')),
                        encode(sha256(decode($1,'hex')),'hex')",
                &[&vector["hex"].as_str().unwrap()],
            )
            .unwrap();
        assert_eq!(
            row.get::<_, serde_json::Value>(0),
            expected,
            "{}",
            vector["name"]
        );
        assert_eq!(row.get::<_, String>(1), vector["sha256"].as_str().unwrap());
    }
}

#[test]
fn sql_rejects_truncated_trailing_noncanonical_and_oversized_hearing_values() {
    let Some(mut db) = Fixture::new() else { return };
    let vectors = vectors();
    let valid = vectors[0]["hex"].as_str().unwrap();
    for invalid in [
        String::new(),
        valid[..valid.len() - 2].to_owned(),
        format!("{valid}00"),
        valid.replacen("07436f7572742041", "07206f7572742041", 1),
        format!("{}04{}", &valid[..10], &valid[12..]),
        format!("{}00000001{}", &valid[..28], &valid[36..]),
        "00".repeat(10727),
    ] {
        let error = db
            .admin
            .query_one("SELECT hearing_values(decode($1,'hex'))", &[&invalid])
            .unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}
