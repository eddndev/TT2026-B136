use crate::member_support;
use infrastructure::PostgresMemberStore;
use member_support::{store, Fixture};
use std::sync::Arc;

#[test]
fn member_schema_requires_generation_and_rejects_excessive_runtime_grants() {
    let Some(mut db) = Fixture::new() else { return };
    let present: bool = db.admin.query_one("SELECT EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='users'::regclass AND attname='auth_generation' AND NOT attisdropped)", &[]).unwrap().get(0);
    assert!(present, "missing durable authentication generation");
    store(&db);
    db.admin
        .batch_execute(&format!("GRANT UPDATE ON users TO {}", db.role))
        .unwrap();
    assert!(
        PostgresMemberStore::open(&db.runtime_url, Arc::new(infrastructure::SystemClock)).is_err()
    );
}

#[test]
fn startup_rejects_a_nonempty_account_inventory_without_an_active_owner() {
    let Some(mut db) = Fixture::new() else { return };
    store(&db);
    db.admin.batch_execute("ALTER TABLE users DISABLE TRIGGER USER; UPDATE users SET active=false; ALTER TABLE users ENABLE TRIGGER USER").unwrap();
    assert!(
        PostgresMemberStore::open(&db.runtime_url, Arc::new(infrastructure::SystemClock)).is_err()
    );
}

#[test]
fn display_names_are_required_last_insert_only_and_checked() {
    let Some(mut db) = Fixture::new() else { return };
    let row = db
        .admin
        .query_one(
            "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
            EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
        FROM pg_attribute a WHERE a.attrelid='users'::regclass AND a.attnum>0
            AND NOT a.attisdropped ORDER BY a.attnum DESC LIMIT 1",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "display_name");
    assert_eq!(row.get::<_, String>(1), "text");
    assert!(row.get::<_, bool>(2), "display names must be required");
    assert!(!row.get::<_, bool>(3), "display names must have no default");
    let checked: bool = db
        .admin
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='users'::regclass
            AND conname='member_display_name_check' AND contype='c' AND convalidated)",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(checked, "missing display name check");
    let grants = db
        .admin
        .query_one(
            "SELECT has_column_privilege($1,'users','display_name','INSERT'),
            has_column_privilege($1,'users','display_name','UPDATE')",
            &[&db.role],
        )
        .unwrap();
    assert!(grants.get::<_, bool>(0), "runtime role must insert names");
    assert!(!grants.get::<_, bool>(1), "names are immutable at runtime");
    store(&db);
    db.admin
        .batch_execute(&format!(
            "GRANT UPDATE(display_name) ON users TO {}",
            db.role
        ))
        .unwrap();
    assert!(
        PostgresMemberStore::open(&db.runtime_url, Arc::new(infrastructure::SystemClock)).is_err()
    );
}
