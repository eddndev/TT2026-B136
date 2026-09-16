mod case_administration_support;
use case_administration_support::Fixture;

#[test]
fn sql_decodes_all_independent_subject_and_role_vectors_without_normalization() {
    let Some(mut db) = Fixture::new() else { return };
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/typed_participant_vectors.json"
    ))
    .unwrap();
    for row in rows {
        let name = row["name"].as_str().unwrap();
        if name.starts_with("credential_") {
            continue;
        }
        let is_subject = row["input"].get("subject").is_none();
        let sql = if is_subject {
            "SELECT typed_subject_values(decode($1,'hex')),encode(sha256(decode($1,'hex')),'hex')"
        } else {
            "SELECT typed_participant_values(decode($1,'hex')),encode(sha256(decode($1,'hex')),'hex')"
        };
        let result = db
            .admin
            .query_one(sql, &[&row["hex"].as_str().unwrap()])
            .unwrap();
        assert_eq!(
            result.get::<_, serde_json::Value>(0),
            row["input"],
            "{name}"
        );
        assert_eq!(
            result.get::<_, String>(1),
            row["sha256"].as_str().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn sql_rejects_truncated_trailing_noncanonical_and_oversized_canonical_values() {
    let Some(mut db) = Fixture::new() else { return };
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/typed_participant_vectors.json"
    ))
    .unwrap();
    let valid = rows[0]["hex"].as_str().unwrap();
    for invalid in [
        String::new(),
        valid[..valid.len() - 2].to_owned(),
        format!("{valid}00"),
        valid.replacen("03416e61", "03206e61", 1),
        "00".repeat(5677),
    ] {
        let error = db
            .admin
            .query_one("SELECT typed_subject_values(decode($1,'hex'))", &[&invalid])
            .unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}
