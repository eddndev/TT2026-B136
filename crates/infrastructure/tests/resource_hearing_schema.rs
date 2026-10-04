mod case_administration_support;

use case_administration_support::Fixture;
use postgres::error::SqlState;

const TABLES: [&str; 2] = ["case_resource_hearings", "case_resource_hearing_revisions"];

#[test]
fn migration_adds_no_hearings_and_preserves_catalog_identity_on_repetition() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    let before = db.snapshot();
    let constraints = constraint_inventory(&mut db);
    db.migrate();
    assert_eq!(db.snapshot(), before);
    assert_eq!(constraint_inventory(&mut db), constraints);
    for table in TABLES {
        let count: i64 = db
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0, "migration invented a resource hearing");
        let denied = db
            .runtime()
            .batch_execute(&format!("DELETE FROM {table}"))
            .unwrap_err();
        assert_eq!(denied.code(), Some(&SqlState::INSUFFICIENT_PRIVILEGE));
        let immutable = db
            .admin
            .batch_execute(&format!("UPDATE {table} SET case_id=case_id"))
            .unwrap_err();
        assert_eq!(immutable.code(), Some(&SqlState::CHECK_VIOLATION));
    }
    open(&db).unwrap();
}

#[test]
fn startup_rejects_changed_hearing_columns_checks_indexes_and_guards() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    for (damage, repair) in [
        (
            "ALTER TABLE case_resource_hearings ADD COLUMN extra boolean",
            "ALTER TABLE case_resource_hearings DROP COLUMN extra",
        ),
        (
            "ALTER TABLE case_resource_hearings ALTER COLUMN initial_revision SET DEFAULT 2",
            "ALTER TABLE case_resource_hearings ALTER COLUMN initial_revision SET DEFAULT 1",
        ),
        (
            "ALTER TABLE case_resource_hearing_revisions DISABLE TRIGGER USER",
            "ALTER TABLE case_resource_hearing_revisions ENABLE TRIGGER USER",
        ),
        (
            "ALTER INDEX resource_hearing_case_resource_order RENAME TO altered_hearing_order",
            "ALTER INDEX altered_hearing_order RENAME TO resource_hearing_case_resource_order",
        ),
        (
            "ALTER TABLE case_resource_hearings DROP CONSTRAINT case_resource_hearings_initial_revision_check",
            "ALTER TABLE case_resource_hearings ADD CONSTRAINT case_resource_hearings_initial_revision_check CHECK(initial_revision=1)",
        ),
    ] {
        db.admin.batch_execute(damage).unwrap();
        assert!(open(&db).is_err(), "accepted altered hearing schema: {damage}");
        db.admin.batch_execute(repair).unwrap();
        open(&db).unwrap();
    }
}

#[test]
fn startup_rejects_replaced_hearing_guard_and_excessive_runtime_grants() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    let original: String = db
        .admin
        .query_one(
            "SELECT pg_get_functiondef('enforce_resource_hearing_sequence()'::regprocedure)",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin.batch_execute("CREATE OR REPLACE FUNCTION enforce_resource_hearing_sequence() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$").unwrap();
    assert!(open(&db).is_err(), "accepted a guard without authorization");
    db.admin.batch_execute(&original).unwrap();
    open(&db).unwrap();
    for table in TABLES {
        db.admin
            .batch_execute(&format!("GRANT SELECT ON {table} TO PUBLIC"))
            .unwrap();
        assert!(
            open(&db).is_err(),
            "accepted public resource hearing access"
        );
        db.admin
            .batch_execute(&format!("REVOKE SELECT ON {table} FROM PUBLIC"))
            .unwrap();
        open(&db).unwrap();
        db.admin
            .batch_execute(&format!("GRANT UPDATE ON {table} TO {}", db.role))
            .unwrap();
        assert!(open(&db).is_err(), "accepted mutable hearing history");
        db.admin
            .batch_execute(&format!("REVOKE UPDATE ON {table} FROM {}", db.role))
            .unwrap();
        open(&db).unwrap();
    }
}

#[test]
fn association_resource_hearing_target_requires_its_exact_foreign_capture() {
    let Some(mut db) = Fixture::new() else { return };
    require_tables(&mut db);
    let columns: Vec<String> = db
        .admin
        .query(
            "SELECT attname::text FROM pg_attribute
             WHERE attrelid='case_resource_activity_association_revisions'::regclass
               AND attname IN ('resource_hearing_id','resource_hearing_revision',
                               'resource_hearing_capture_digest')
               AND NOT attisdropped AND NOT attnotnull ORDER BY attname",
            &[],
        )
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(
        columns,
        [
            "resource_hearing_capture_digest",
            "resource_hearing_id",
            "resource_hearing_revision",
        ]
    );
    for name in [
        "resource_activity_resource_hearing_root",
        "resource_activity_resource_hearing_capture",
        "resource_activity_target_shape",
    ] {
        let definition: String = db
            .admin
            .query_one(
                "SELECT pg_get_constraintdef(oid) FROM pg_constraint
                 WHERE conrelid='case_resource_activity_association_revisions'::regclass
                   AND conname=$1",
                &[&name],
            )
            .unwrap()
            .get(0);
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE case_resource_activity_association_revisions DROP CONSTRAINT {name}"
            ))
            .unwrap();
        assert!(
            open(&db).is_err(),
            "accepted missing target boundary: {name}"
        );
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE case_resource_activity_association_revisions ADD CONSTRAINT {name} {definition}"
            ))
            .unwrap();
        open(&db).unwrap();
    }
}

fn require_tables(db: &mut Fixture) {
    for table in TABLES {
        let found: Option<String> = db
            .admin
            .query_one("SELECT to_regclass($1)::text", &[&table])
            .unwrap()
            .get(0);
        assert!(found.is_some(), "missing resource hearing table: {table}");
    }
}

fn constraint_inventory(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            "SELECT coalesce(jsonb_agg(jsonb_build_array(c.oid,c.conname,
             pg_get_constraintdef(c.oid)) ORDER BY c.oid),'[]'::jsonb)
         FROM pg_constraint c WHERE c.conrelid IN (
             'case_resource_hearings'::regclass,
             'case_resource_hearing_revisions'::regclass,
             'case_resource_activity_association_revisions'::regclass)",
            &[],
        )
        .unwrap()
        .get(0)
}

fn open(
    db: &Fixture,
) -> Result<infrastructure::PostgresCaseRepository, application::ApplicationError> {
    infrastructure::PostgresCaseRepository::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
    )
}
