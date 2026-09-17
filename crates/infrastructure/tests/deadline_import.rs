#[allow(dead_code)]
mod legacy_database_support;
use legacy_database_support::{Database, Source};
use uuid::Uuid;

#[test]
fn initial_import_rejects_an_unaudited_deadline_root_from_a_partial_restore() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::new();
    db.seed_case(source.case_id);
    source.inspect().check_target(&db.url).unwrap();
    db.client
        .batch_execute("SET session_replication_role=replica")
        .unwrap();
    db.client
        .execute(
            "INSERT INTO case_deadlines(id,case_id) VALUES($1,$2)",
            &[&Uuid::nil(), &source.case_id.as_uuid()],
        )
        .unwrap();
    db.client
        .batch_execute("SET session_replication_role=origin")
        .unwrap();
    let before = db.stored_state();
    let document = std::fs::read(source.document_path()).unwrap();
    let audit = std::fs::read(source.audit_path()).unwrap();
    assert!(
        source.inspect().check_target(&db.url).is_err(),
        "deadline rows must reject an initial import"
    );
    assert!(source.inspect().apply(&db.url).is_err());
    assert_eq!(db.stored_state(), before);
    assert_eq!(
        db.client
            .query_one("SELECT count(*) FROM case_deadlines", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    assert_eq!(std::fs::read(source.document_path()).unwrap(), document);
    assert_eq!(std::fs::read(source.audit_path()).unwrap(), audit);
    source.assert_unmarked();
}
