mod case_administration_support;

use case_administration_support::Fixture;

#[test]
fn migration_adds_empty_hearing_history_without_fabricating_legacy_appointments() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["case_hearings", "case_hearing_revisions"] {
        let exists: bool = db
            .admin
            .query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])
            .unwrap()
            .get(0);
        assert!(exists, "hearing migration must install {table}");
        let count: i64 = db
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0, "legacy cases must not receive invented hearings");
    }
    let before = db.snapshot();
    db.migrate();
    assert_eq!(db.snapshot(), before);
    let counts = db
        .admin
        .query_one(
            "SELECT (SELECT count(*) FROM case_hearings),
                    (SELECT count(*) FROM case_hearing_revisions)",
            &[],
        )
        .unwrap();
    assert_eq!(counts.get::<_, i64>(0), 0);
    assert_eq!(counts.get::<_, i64>(1), 0);
}

#[test]
fn runtime_can_read_and_append_but_cannot_rewrite_hearing_history() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["case_hearings", "case_hearing_revisions"] {
        let row = db
            .admin
            .query_one(
                "SELECT has_table_privilege($1,$2,'SELECT'),
                    has_table_privilege($1,$2,'INSERT'),
                    has_table_privilege($1,$2,'UPDATE,DELETE,TRUNCATE,TRIGGER')",
                &[&db.role, &table],
            )
            .unwrap();
        assert!(row.get::<_, bool>(0), "runtime must read {table}");
        assert!(row.get::<_, bool>(1), "runtime must append {table}");
        assert!(!row.get::<_, bool>(2), "runtime must preserve {table}");
    }
    let allowed: bool = db
        .admin
        .query_one(
            "SELECT has_function_privilege($1,'hearing_values(bytea)','EXECUTE')",
            &[&db.role],
        )
        .unwrap()
        .get(0);
    assert!(allowed);
}

#[test]
fn startup_rejects_missing_or_replaced_hearing_schema_checks() {
    for alteration in [
        "ALTER TABLE case_hearing_revisions DROP CONSTRAINT hearing_values_hash",
        "ALTER TABLE case_hearing_revisions RENAME CONSTRAINT hearing_values_hash TO unrelated_hash",
        "ALTER TABLE case_hearing_revisions ALTER COLUMN values_view DROP EXPRESSION",
        "ALTER TABLE case_hearing_revisions ALTER COLUMN operation_id DROP NOT NULL",
        "ALTER TABLE case_hearing_revisions DROP CONSTRAINT hearing_operation_unique",
        "ALTER TABLE case_hearings DROP CONSTRAINT hearing_first_revision",
        "DROP TRIGGER hearing_sequence ON case_hearing_revisions",
        "ALTER TABLE case_hearing_revisions DISABLE TRIGGER hearing_sequence",
        "ALTER FUNCTION enforce_hearing_sequence() SECURITY DEFINER",
        "ALTER TABLE case_hearing_revisions DISABLE TRIGGER hearing_immutable",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.store();
        db.admin.batch_execute(alteration).unwrap();
        assert!(infrastructure::PostgresCaseRepository::open(
            &db.runtime_url, std::sync::Arc::new(infrastructure::RingSha256Hasher)
        ).is_err(), "{alteration}");
    }
}

#[test]
fn startup_rejects_history_update_privileges_and_an_orphan_root() {
    let Some(mut db) = Fixture::new() else { return };
    db.admin
        .batch_execute(&format!(
            "GRANT UPDATE ON case_hearing_revisions TO {}",
            db.role
        ))
        .unwrap();
    assert!(infrastructure::PostgresCaseRepository::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher)
    )
    .is_err());
    db.migrate();
    db.admin
        .batch_execute("ALTER TABLE case_hearings DISABLE TRIGGER ALL")
        .unwrap();
    db.admin
        .execute(
            "INSERT INTO case_hearings(id,case_id) VALUES($1,$2)",
            &[&uuid::Uuid::new_v4(), &db.case.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_hearings ENABLE TRIGGER ALL")
        .unwrap();
    assert!(infrastructure::PostgresCaseRepository::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher)
    )
    .is_err());
}

#[test]
fn hearing_history_is_immutable_even_for_privileged_empty_table_mutations() {
    let Some(mut db) = Fixture::new() else { return };
    for statement in [
        "DELETE FROM case_hearing_revisions",
        "UPDATE case_hearings SET id=id",
        "TRUNCATE case_hearings CASCADE",
    ] {
        let failure = db.admin.batch_execute(statement).unwrap_err();
        assert_eq!(
            failure.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}
