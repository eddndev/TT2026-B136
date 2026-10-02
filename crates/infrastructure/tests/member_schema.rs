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
