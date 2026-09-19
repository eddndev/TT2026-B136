mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod procedural_fact_backend_support;
mod procedural_resource_support;
mod resource_activity_support;
use application::{resource_activities::*, ApplicationError};
use domain::identity::Role;
use resource_activity_support::*;

#[test]
fn startup_and_open_connection_reject_tampered_association_captures_without_writes() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let linked = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        captures.resource.id,
        captures.link(),
    );
    let adapter = store(&db);
    let before = business_and_audit(&mut db);
    drop(store(&db));
    assert_eq!(business_and_audit(&mut db), before);
    db.admin
        .batch_execute(
            "ALTER TABLE case_resource_activity_association_revisions DISABLE TRIGGER USER",
        )
        .unwrap();
    db.admin.execute("UPDATE case_resource_activity_association_revisions SET recorded_by_email='forged@example.test' WHERE association_id=$1", &[&linked.id.as_uuid()]).unwrap();
    db.admin
        .batch_execute(
            "ALTER TABLE case_resource_activity_association_revisions ENABLE TRIGGER USER",
        )
        .unwrap();
    let corrupt = business_and_audit(&mut db);
    assert!(matches!(
        adapter.get(
            db.owner,
            db.case,
            captures.resource.id,
            linked.id,
            None,
            db.at
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::StoredInconsistent(_)
        ))
    ));
    assert!(matches!(
        adapter.history(
            db.owner,
            db.case,
            captures.resource.id,
            linked.id,
            history_query(),
            db.at
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::StoredInconsistent(_)
        ))
    ));
    assert!(infrastructure::PostgresCaseRepository::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher)
    )
    .is_err());
    assert_eq!(business_and_audit(&mut db), corrupt);
    db.admin
        .batch_execute(
            "ALTER TABLE case_resource_activity_association_revisions DISABLE TRIGGER USER",
        )
        .unwrap();
    db.admin.execute("UPDATE case_resource_activity_association_revisions SET recorded_by_email=$2 WHERE association_id=$1", &[&linked.id.as_uuid(), &linked.recorded_by.email]).unwrap();
    db.admin
        .batch_execute(
            "ALTER TABLE case_resource_activity_association_revisions ENABLE TRIGGER USER",
        )
        .unwrap();
    drop(store(&db));
    assert_eq!(business_and_audit(&mut db), before);
}
