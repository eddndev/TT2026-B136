mod case_administration_support;
use case_administration_support::Fixture;
use infrastructure::PostgresCaseDocumentStore;

#[test]
fn incident_schema_is_append_only_and_rejects_catalog_or_runtime_grant_changes() {
    let Some(mut db) = Fixture::new() else { return };
    let table: Option<String> = db
        .admin
        .query_one(
            "SELECT to_regclass('document_integrity_incidents')::text",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(table.is_some(), "missing document integrity incident table");
    let count: i64 = db
        .admin
        .query_one("SELECT count(*) FROM document_integrity_incidents", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
    let denied = db
        .runtime()
        .batch_execute("DELETE FROM document_integrity_incidents")
        .unwrap_err();
    assert_eq!(
        denied.code(),
        Some(&postgres::error::SqlState::INSUFFICIENT_PRIVILEGE)
    );
    db.admin
        .batch_execute("ALTER TABLE document_integrity_incidents ADD COLUMN extra boolean")
        .unwrap();
    assert!(PostgresCaseDocumentStore::open(&db.runtime_url).is_err());
    db.admin
        .batch_execute("ALTER TABLE document_integrity_incidents DROP COLUMN extra")
        .unwrap();
    PostgresCaseDocumentStore::open(&db.runtime_url).unwrap();
    db.admin
        .batch_execute(&format!(
            "GRANT UPDATE ON document_integrity_incidents TO {}",
            db.role
        ))
        .unwrap();
    assert!(PostgresCaseDocumentStore::open(&db.runtime_url).is_err());
    db.admin
        .batch_execute(&format!(
            "REVOKE UPDATE ON document_integrity_incidents FROM {}",
            db.role
        ))
        .unwrap();
    PostgresCaseDocumentStore::open(&db.runtime_url).unwrap();
}
