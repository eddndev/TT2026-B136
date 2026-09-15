mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;

use application::case_stages::*;
use application::ApplicationError;
use case_stage_database_support::*;
use domain::identity::Role;
use infrastructure::{PostgresCaseStageStore, RingSha256Hasher};
use std::sync::Arc;

#[test]
fn startup_rejects_disabled_or_missing_stage_guards() {
    for defect in [
        "ALTER TABLE case_stage_revisions DISABLE TRIGGER case_stage_sequence",
        "ALTER TABLE case_stage_revisions DROP CONSTRAINT case_stage_digest",
        "ALTER TABLE case_initial_stage_registrations DISABLE TRIGGER case_stage_initial_exclusive",
        "ALTER TABLE case_stage_revisions ALTER COLUMN support_name DROP NOT NULL",
        "ALTER FUNCTION case_stage_values_bytes(case_stage_revisions) SECURITY DEFINER",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin.batch_execute(defect).unwrap();
        assert!(
            matches!(
                PostgresCaseStageStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)),
                Err(ApplicationError::InvalidConfiguration(_))
            ),
            "startup accepted: {defect}"
        );
    }
}

#[test]
fn startup_rejects_discontinuous_inventory_even_when_guards_are_restored() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    service(&db, db.owner, Role::Owner, FormatCheck(None))
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Investigation),
        )
        .unwrap();
    db.admin.batch_execute("ALTER TABLE case_stage_revisions DISABLE TRIGGER case_stage_sequence; INSERT INTO case_stage_revisions SELECT (jsonb_populate_record(s,'{\"revision\":9}'::jsonb)).* FROM case_stage_revisions s; ALTER TABLE case_stage_revisions ENABLE TRIGGER case_stage_sequence").unwrap();
    assert!(matches!(
        PostgresCaseStageStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}

#[test]
fn concurrent_seal_is_reported_as_changed_before_loading_oversized_evidence() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    let before = snapshot(&mut db);
    let url = db.admin_url.clone();
    let id = record.id.as_uuid();
    let format = FormatCheck(Some(Box::new(move || {
        let mut connection = postgres::Client::connect(&url, postgres::NoTls).unwrap();
        connection.execute("UPDATE documents SET evidence=jsonb_build_object('large',repeat('x',1048577)) WHERE id=$1",&[&id]).unwrap();
    })));
    let result = service(&db, db.owner, Role::Owner, format).adopt(
        "session",
        db.case,
        CaseStageExpectation::Unregistered,
        adoption(&db, &record, CaseStage::Investigation),
    );
    assert!(
        matches!(result, Err(ApplicationError::StageSupportChanged)),
        "{result:?}"
    );
    assert_eq!(snapshot(&mut db), before);
}
