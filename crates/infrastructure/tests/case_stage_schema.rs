mod case_administration_support;

use case_administration_support::Fixture;

#[test]
fn migration_adds_empty_stage_history_without_copying_initial_registrations() {
    let Some(mut db) = Fixture::new() else { return };
    let exists: bool = db
        .admin
        .query_one(
            "SELECT to_regclass('case_stage_revisions') IS NOT NULL",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(
        exists,
        "stage migration must install its immutable history table"
    );
    let count: i64 = db
        .admin
        .query_one("SELECT count(*) FROM case_stage_revisions", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
    let before = db.snapshot();
    db.migrate();
    assert_eq!(db.snapshot(), before);
}

#[test]
fn sql_declared_time_preserves_precision_offsets_and_nanoseconds() {
    let Some(mut db) = Fixture::new() else { return };
    let bounds: Vec<i64> = db
        .admin
        .query_one(
            "SELECT case_stage_time_bounds('date','2026-09-15'::date,NULL,NULL,-21600)",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(bounds, vec![1_789_452_000, 0, 1_789_538_399, 999_999_999]);
    let exact: Vec<i64> = db
        .admin
        .query_one(
            "SELECT case_stage_time_bounds('instant',NULL,1789452000,123456789,-21600)",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(
        exact,
        vec![1_789_452_000, 123_456_789, 1_789_452_000, 123_456_789]
    );
    let invalid: bool = db.admin.query_one("SELECT case_stage_time_bounds('date','0001-01-01'::date,NULL,NULL,60) IS NULL AND case_stage_time_bounds('date','9999-12-31'::date,NULL,NULL,-60) IS NULL AND case_stage_time_bounds('instant',NULL,1789452000,0,1) IS NULL AND case_stage_time_bounds('instant',NULL,1789452000,0,50460) IS NULL", &[]).unwrap().get(0);
    assert!(invalid);
}

#[test]
fn sql_canonical_adoption_matches_exact_rust_vector_and_native_sha256() {
    let Some(mut db) = Fixture::new() else { return };
    let value = serde_json::json!({
        "change_kind":"adoption", "stage":"trial",
        "act_precision":"date", "act_date":"2026-09-14", "act_offset_seconds":-21600,
        "reason":"Legado\nVerificado",
        "support_id":"00112233-4455-6677-8899-aabbccddeeff", "support_version":7,
        "support_digest":"\\x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        "support_name":"support.pdf", "support_format":"pdf", "support_policy":"pdf_docx_v1"
    });
    let row = db.admin.query_one("SELECT encode(case_stage_values_bytes(jsonb_populate_record(NULL::case_stage_revisions,$1)),'hex'),encode(sha256(case_stage_values_bytes(jsonb_populate_record(NULL::case_stage_revisions,$1))),'hex')", &[&value]).unwrap();
    assert_eq!(
        row.get::<_, String>(0),
        concat!(
            "435354473100020007ea090effffaba0000000114c656761646f0a5665726966696361646f",
            "00112233445566778899aabbccddeeff00000007",
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        )
    );
    assert_eq!(
        row.get::<_, String>(1),
        "b19b426504fecb2045bb4150b2051fa8e5d7dbca46270b974c3a6b5a0e490889"
    );
}

#[test]
fn duplicate_support_roles_require_the_same_complete_snapshot() {
    let Some(mut db) = Fixture::new() else { return };
    let mut value = serde_json::json!({
        "change_kind":"to_trial", "from_stage":"intermediate", "stage":"trial",
        "act_precision":"date", "act_date":"2024-12-31", "act_offset_seconds":0,
        "received_precision":"date", "received_date":"2024-12-31", "received_offset_seconds":0,
        "receiving_court":"Court", "support_id":"00112233-4455-6677-8899-aabbccddeeff",
        "support_version":7, "support_digest":format!("\\x{}","aa".repeat(32)),
        "support_name":"support.pdf", "support_format":"pdf", "support_policy":"pdf_docx_v1"
    });
    for field in ["id", "version", "digest", "name", "format", "policy"] {
        value[format!("receipt_{field}")] = value[format!("support_{field}")].clone();
    }
    let valid = |db: &mut Fixture, value: &serde_json::Value| -> bool {
        db.admin.query_one("SELECT case_stage_values_canonical(jsonb_populate_record(NULL::case_stage_revisions,$1))", &[value]).unwrap().get(0)
    };
    assert!(valid(&mut db, &value));
    value["receipt_format"] = "docx".into();
    assert!(
        !valid(&mut db, &value),
        "one exact support cannot have two admitted formats"
    );
    value["receipt_format"] = "pdf".into();
    value["receipt_name"] = "different.pdf".into();
    assert!(
        !valid(&mut db, &value),
        "one exact support cannot have two captured names"
    );
}
