mod document_content_support;
use application::document_integrity::DocumentIntegrityStore;
use application::ApplicationError;
use document_content_support::Fixture;
use infrastructure::PostgresCaseDocumentStore;

#[test]
fn altered_incident_projection_is_rejected_on_read_and_reopen() {
    let Some(mut f) = Fixture::new() else { return };
    let receipt = f.store.record_rejection(&f.observation()).unwrap();
    f.db.admin
        .batch_execute("ALTER TABLE document_integrity_incidents DISABLE TRIGGER USER")
        .unwrap();
    f.db.admin
        .execute(
            "UPDATE document_integrity_incidents SET failure='malformed_vault' WHERE id=$1",
            &[&receipt.incident_id.as_uuid()],
        )
        .unwrap();
    f.db.admin
        .batch_execute("ALTER TABLE document_integrity_incidents ENABLE TRIGGER USER")
        .unwrap();
    assert!(matches!(
        f.store.get(f.db.owner, receipt.incident_id, f.db.at),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert!(matches!(
        PostgresCaseDocumentStore::open(&f.db.runtime_url),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert_eq!(f.content_events(), 0);
}
