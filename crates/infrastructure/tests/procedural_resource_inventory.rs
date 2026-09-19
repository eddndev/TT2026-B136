mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
mod procedural_resource_support;
use domain::identity::Role;
use procedural_resource_support::*;

#[test]
fn startup_replays_receipts_without_writes_and_rejects_view_substitution() {
    let Some(mut db) = Fixture::new() else { return };
    let first = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        registration(&db),
    );
    let before = snapshot(&mut db);
    drop(store(&db));
    assert_eq!(snapshot(&mut db), before);
    let original: serde_json::Value = db
        .admin
        .query_one(
            "SELECT values_view FROM case_procedural_resource_revisions WHERE resource_id=$1",
            &[&first.id.as_uuid()],
        )
        .unwrap()
        .get(0);
    db.admin
        .batch_execute("ALTER TABLE case_procedural_resource_revisions DISABLE TRIGGER USER")
        .unwrap();
    db.admin
        .execute(
            "UPDATE case_procedural_resource_revisions SET values_view='{}' WHERE resource_id=$1",
            &[&first.id.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_procedural_resource_revisions ENABLE TRIGGER USER")
        .unwrap();
    assert!(infrastructure::PostgresCaseRepository::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher)
    )
    .is_err());
    db.admin
        .batch_execute("ALTER TABLE case_procedural_resource_revisions DISABLE TRIGGER USER")
        .unwrap();
    db.admin
        .execute(
            "UPDATE case_procedural_resource_revisions SET values_view=$2 WHERE resource_id=$1",
            &[&first.id.as_uuid(), &original],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_procedural_resource_revisions ENABLE TRIGGER USER")
        .unwrap();
    drop(store(&db));
    assert_eq!(snapshot(&mut db), before);
}
