mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;

use domain::identity::Role;
use procedural_fact_backend_support::*;

#[test]
fn administration_shape_rejects_null_partial_and_mixed_representations() {
    let Some(mut db) = Fixture::new() else { return };
    persist(&service(&db, db.owner, Role::Owner), db.case, record());
    db.admin
        .batch_execute(
            "ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER procedural_fact_immutable",
        )
        .unwrap();
    for fields in [
        "recorded_administration_revision=NULL, recorded_administration_digest=decode(repeat('00',32),'hex'),
         recorded_administration_title=NULL, recorded_administration_reference=NULL",
        "recorded_administration_revision=NULL, recorded_administration_digest=NULL,
         recorded_administration_title=NULL, recorded_administration_reference=NULL",
        "recorded_administration_revision=1, recorded_administration_digest=NULL,
         recorded_administration_title=NULL, recorded_administration_reference=NULL",
        "recorded_administration_revision=1, recorded_administration_digest=decode(repeat('00',32),'hex'),
         recorded_administration_title='Mixed baseline', recorded_administration_reference='Mixed baseline'",
        "recorded_administration_revision=NULL, recorded_administration_digest=NULL,
         recorded_administration_title='Partial baseline', recorded_administration_reference=NULL",
    ] {
        let error = db.admin.batch_execute(&format!(
            "UPDATE case_procedural_fact_revisions SET {fields}"
        )).unwrap_err();
        let database = error.as_db_error().unwrap();
        assert_eq!(database.code(), &postgres::error::SqlState::CHECK_VIOLATION);
        assert_eq!(database.constraint(), Some("procedural_fact_administration_shape"), "{fields}");
    }
    db.admin
        .batch_execute(
            "ALTER TABLE case_procedural_fact_revisions ENABLE TRIGGER procedural_fact_immutable",
        )
        .unwrap();
    db.store();
}
