mod case_administration_support;
use case_administration_support::Fixture;

#[test]
fn startup_rejects_changed_columns_checks_indexes_guards_and_permissions() {
    let Some(mut db) = Fixture::new() else { return };
    for (damage, repair) in [
        ("ALTER TABLE case_procedural_resources ADD COLUMN extra boolean", "ALTER TABLE case_procedural_resources DROP COLUMN extra"),
        ("ALTER TABLE case_procedural_resources ALTER COLUMN initial_revision SET DEFAULT 2", "ALTER TABLE case_procedural_resources ALTER COLUMN initial_revision SET DEFAULT 1"),
        ("ALTER TABLE case_procedural_resources DISABLE TRIGGER procedural_resource_immutable", "ALTER TABLE case_procedural_resources ENABLE TRIGGER procedural_resource_immutable"),
        ("ALTER INDEX procedural_resource_case_order RENAME TO wrong_resource_index", "ALTER INDEX wrong_resource_index RENAME TO procedural_resource_case_order"),
        ("ALTER TABLE case_procedural_resources DROP CONSTRAINT case_procedural_resources_initial_revision_check", "ALTER TABLE case_procedural_resources ADD CONSTRAINT case_procedural_resources_initial_revision_check CHECK(initial_revision=1)"),
        ("GRANT SELECT ON case_procedural_resources TO PUBLIC", "REVOKE SELECT ON case_procedural_resources FROM PUBLIC"),
    ] {
        db.admin.batch_execute(damage).unwrap();
        assert!(open(&db).is_err(), "accepted altered catalog: {damage}");
        db.admin.batch_execute(repair).unwrap();
        open(&db).unwrap();
    }
    let original: String = db
        .admin
        .query_one(
            "SELECT pg_get_functiondef('preserve_procedural_resource_history()'::regprocedure)",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin.batch_execute("CREATE OR REPLACE FUNCTION preserve_procedural_resource_history() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NULL; END; $$").unwrap();
    assert!(open(&db).is_err());
    db.admin.batch_execute(&original).unwrap();
    open(&db).unwrap();
    db.admin
        .batch_execute(&format!(
            "GRANT UPDATE ON case_procedural_resources TO {}",
            db.role
        ))
        .unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute(&format!(
            "REVOKE UPDATE ON case_procedural_resources FROM {}",
            db.role
        ))
        .unwrap();
    open(&db).unwrap();
}

#[test]
fn migration_invents_no_resources_and_runtime_cannot_rewrite_history() {
    let Some(mut db) = Fixture::new() else { return };
    let before = db.snapshot();
    db.migrate();
    assert_eq!(db.snapshot(), before);
    for table in [
        "case_procedural_resources",
        "case_procedural_resource_acts",
        "case_procedural_resource_revisions",
    ] {
        let count: i64 = db
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0);
        let denied = db
            .runtime()
            .batch_execute(&format!("DELETE FROM {table}"))
            .unwrap_err();
        assert_eq!(
            denied.code(),
            Some(&postgres::error::SqlState::INSUFFICIENT_PRIVILEGE)
        );
        let immutable = db
            .admin
            .batch_execute(&format!("UPDATE {table} SET case_id=case_id"))
            .unwrap_err();
        assert_eq!(
            immutable.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}
fn open(
    db: &Fixture,
) -> Result<infrastructure::PostgresCaseRepository, application::ApplicationError> {
    infrastructure::PostgresCaseRepository::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
    )
}
