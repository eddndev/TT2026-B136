#[allow(dead_code)]
mod legacy_database_support;
use legacy_database_support::{Database, Source};

#[test]
fn initial_import_rejects_hearing_roots_even_when_other_history_is_empty() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::new();
    db.seed_case(source.case_id);
    db.client
        .batch_execute("ALTER TABLE case_hearings DISABLE TRIGGER ALL")
        .unwrap();
    db.client
        .execute(
            "INSERT INTO case_hearings(id,case_id) VALUES($1,$2)",
            &[&uuid::Uuid::new_v4(), &source.case_id.as_uuid()],
        )
        .unwrap();
    db.client
        .batch_execute("ALTER TABLE case_hearings ENABLE TRIGGER ALL")
        .unwrap();
    let before = db.stored_state();
    assert!(source.inspect().check_target(&db.url).is_err());
    assert!(source.inspect().apply(&db.url).is_err());
    assert_eq!(db.stored_state(), before);
    source.assert_unmarked();
}
