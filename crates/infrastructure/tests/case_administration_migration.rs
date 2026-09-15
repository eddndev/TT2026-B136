mod case_administration_support;

use case_administration_support::Fixture;
use serde_json::Value;

#[test]
fn canonical_sql_matches_fixed_utf8_length_and_sha256_vectors_with_empty_search_path() {
    let Some(mut db) = Fixture::new() else {
        return;
    };
    let vectors: Value = serde_json::from_str(include_str!(
        "case_administration_support/canonical_vectors.json"
    ))
    .unwrap();
    db.admin.batch_execute("SET search_path=''").unwrap();
    for vector in vectors["samples"].as_array().unwrap() {
        let profile = &vector["profile"];
        let text = |field: &str| profile[field].as_str();
        let offenses = profile["offenses"].as_array().map(|items| {
            items
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<Vec<_>>()
        });
        let status = if vector["status"] == 0 {
            "active"
        } else {
            "closed"
        };
        let sql = format!("SELECT encode({0}.case_administration_bytes($1,$2,$3,$4,$5,$6,$7,$8,$9,$10),'hex'),encode(pg_catalog.sha256({0}.case_administration_bytes($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)),'hex')", db.schema);
        let row = db
            .admin
            .query_one(
                &sql,
                &[
                    &status,
                    &vector["title"].as_str().unwrap(),
                    &vector["reference"].as_str().unwrap(),
                    &text("nuc"),
                    &text("nuc_authority"),
                    &text("judicial_case_number"),
                    &text("judicial_authority"),
                    &offenses,
                    &text("general_information"),
                    &text("complementary_identifiers"),
                ],
            )
            .unwrap();
        assert_eq!(
            row.get::<_, String>(0),
            vector["canonical_hex"].as_str().unwrap()
        );
        assert_eq!(row.get::<_, String>(1), vector["sha256"].as_str().unwrap());
    }
}

#[test]
fn migration_preserves_baselines_without_fabricated_revisions_and_reapplies_identically() {
    let Some(mut db) = Fixture::old() else {
        return;
    };
    let before = db.root_snapshot();
    db.migrate();
    let marker: Option<i64> = db
        .admin
        .query_one(
            "SELECT required_initial_revision FROM cases WHERE id=$1",
            &[&db.case.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(marker, None);
    let count: i64 = db.admin.query_one("SELECT (SELECT COUNT(*) FROM case_administration_revisions)+(SELECT COUNT(*) FROM case_initial_stage_registrations)", &[]).unwrap().get(0);
    assert_eq!(count, 0);
    let mut after = db.root_snapshot();
    for case in after["cases"].as_array_mut().unwrap() {
        case.as_object_mut()
            .unwrap()
            .remove("required_initial_revision");
    }
    assert_eq!(before, after);
    let snapshot = db.root_snapshot();
    db.migrate();
    assert_eq!(snapshot, db.root_snapshot());
}

#[test]
fn runtime_cannot_insert_a_baseline_or_commit_a_new_root_without_revision_one() {
    let Some(db) = Fixture::new() else {
        return;
    };
    let mut runtime = db.runtime();
    for column in ["required_initial_revision", "created_at"] {
        let sql = format!("INSERT INTO cases(id,title,reference,created_by,{column}) VALUES($1,'New','NEW',$2,NULL)");
        let error = runtime
            .execute(&sql, &[&uuid::Uuid::new_v4(), &db.owner.as_uuid()])
            .unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::INSUFFICIENT_PRIVILEGE)
        );
    }
    let error = runtime
        .execute(
            "INSERT INTO cases(id,title,reference,created_by) VALUES($1,'New','NEW',$2)",
            &[&uuid::Uuid::new_v4(), &db.owner.as_uuid()],
        )
        .unwrap_err();
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::FOREIGN_KEY_VIOLATION)
    );
}
