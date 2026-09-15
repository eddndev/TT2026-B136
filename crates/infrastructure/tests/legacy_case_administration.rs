#[allow(dead_code)]
mod legacy_database_support;
use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};
use application::documents::CaseDocumentStore;
use domain::identity::UserId;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use legacy_database_support::{Database, Source};
use std::sync::Arc;

#[test]
fn initial_import_rejects_administration_history_even_when_the_audit_store_is_empty() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::new();
    db.seed_case(source.case_id);
    db.client.execute("INSERT INTO case_administration_revisions(case_id,revision,title,reference,administrative_status,values_digest,changed_at,changed_by,changed_by_email) SELECT id,1,title,reference,'active',sha256(case_administration_bytes('active',title,reference,NULL,NULL,NULL,NULL,NULL,NULL,NULL)),'2025-01-01T00:00:00Z',created_by,'old@example.test' FROM cases WHERE id=$1", &[&source.case_id.as_uuid()]).unwrap();
    let before = db.stored_state();
    assert!(source.inspect().check_target(&db.url).is_err());
    assert!(source.inspect().apply(&db.url).is_err());
    assert_eq!(db.stored_state(), before);
    source.assert_unmarked();
}
#[test]
fn completed_import_reconciliation_preserves_later_closure_without_changing_legacy_evidence() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::new();
    db.seed_case(source.case_id);
    source.inspect().apply(&db.url).unwrap();
    let url = db.restricted_url();
    let owner = UserId::from_uuid(
        db.client
            .query_one(
                "SELECT created_by FROM cases WHERE id=$1",
                &[&source.case_id.as_uuid()],
            )
            .unwrap()
            .get(0),
    );
    let cases = PostgresCaseRepository::open(&url, Arc::new(RingSha256Hasher)).unwrap();
    let at = time::OffsetDateTime::now_utc();
    let closed = cases
        .change_administrative_status(
            owner,
            source.case_id,
            CaseRevisionExpectation::new(0),
            CaseAdministrativeStatus::Closed,
            at,
        )
        .unwrap();
    let before = db.stored_state();
    let revisions:serde_json::Value=db.client.query_one("SELECT jsonb_agg(to_jsonb(r) ORDER BY case_id,revision) FROM case_administration_revisions r",&[]).unwrap().get(0);
    source.inspect().check_target(&db.url).unwrap();
    source.inspect().apply(&db.url).unwrap();
    infrastructure::legacy::require_completed_import(source.dir.path(), &url).unwrap();
    assert_eq!(db.stored_state(), before);
    assert_eq!(db.client.query_one("SELECT jsonb_agg(to_jsonb(r) ORDER BY case_id,revision) FROM case_administration_revisions r",&[]).unwrap().get::<_,serde_json::Value>(0),revisions);
    assert_eq!(
        cases.get_administration(owner, source.case_id, at).unwrap(),
        closed
    );
    let audit = infrastructure::PostgresCaseDocumentStore::open(&url)
        .unwrap()
        .audit_entries(owner)
        .unwrap();
    assert_eq!(audit[..source.entries.len()], source.entries);
}
