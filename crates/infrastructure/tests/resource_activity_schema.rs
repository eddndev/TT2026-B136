mod case_administration_support;
use case_administration_support::Fixture;

const TABLES: [&str; 2] = [
    "case_resource_activity_associations",
    "case_resource_activity_association_revisions",
];

#[test]
fn migration_creates_empty_association_history_with_append_only_runtime_access() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    let before = db.snapshot();
    db.migrate();
    assert_eq!(db.snapshot(), before);
    for table in TABLES {
        let count: i64 = db
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0, "migration invented an association");
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

#[test]
fn startup_rejects_association_catalog_and_runtime_permission_changes() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    for (damage, repair) in [
        ("ALTER TABLE case_resource_activity_associations ADD COLUMN extra boolean", "ALTER TABLE case_resource_activity_associations DROP COLUMN extra"),
        ("ALTER TABLE case_resource_activity_associations ALTER COLUMN initial_revision SET DEFAULT 2", "ALTER TABLE case_resource_activity_associations ALTER COLUMN initial_revision SET DEFAULT 1"),
        ("ALTER TABLE case_resource_activity_associations DISABLE TRIGGER USER", "ALTER TABLE case_resource_activity_associations ENABLE TRIGGER USER"),
        ("GRANT SELECT ON case_resource_activity_associations TO PUBLIC", "REVOKE SELECT ON case_resource_activity_associations FROM PUBLIC"),
    ] {
        db.admin.batch_execute(damage).unwrap();
        assert!(open(&db).is_err(), "accepted altered association catalog: {damage}");
        db.admin.batch_execute(repair).unwrap();
        open(&db).unwrap();
    }
    db.admin
        .batch_execute(&format!(
            "GRANT UPDATE ON case_resource_activity_association_revisions TO {}",
            db.role
        ))
        .unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute(&format!(
            "REVOKE UPDATE ON case_resource_activity_association_revisions FROM {}",
            db.role
        ))
        .unwrap();
    open(&db).unwrap();
}

fn require_tables(db: &mut Fixture) {
    for table in TABLES {
        let found: Option<String> = db
            .admin
            .query_one("SELECT to_regclass($1)::text", &[&table])
            .unwrap()
            .get(0);
        assert!(
            found.is_some(),
            "missing immutable association table: {table}"
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
