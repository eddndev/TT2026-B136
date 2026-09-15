mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;

use application::case_stages::*;
use application::cases::CaseRepository;
use application::ApplicationError;
use case_stage_database_support::*;
use domain::{cases::CaseId, identity::Role};

#[test]
fn direct_sql_rejects_gap_stale_context_foreign_support_and_wrong_captured_actor() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    let foreign = CaseId::new();
    db.store()
        .register_penal(db.owner, foreign, creation("Other"), db.at)
        .unwrap();
    let other = upload(&db, foreign, "foreign.pdf");
    let original = row_value(&db, &record);
    for patch in [
        serde_json::json!({"revision":2}),
        serde_json::json!({"administration_revision":2}),
        serde_json::json!({"support_id":other.id.to_string(),"support_name":other.name,"support_digest":format!("\\x{}",other.digest.to_hex())}),
        serde_json::json!({"recorded_by_email":"forged@example.test"}),
    ] {
        let mut value = original.clone();
        for (key, value_patch) in patch.as_object().unwrap() {
            value[key] = value_patch.clone();
        }
        let signed = signed_value(&mut db.admin, &value);
        assert!(db.runtime().execute(INSERT, &[&signed]).is_err());
    }
    let count: i64 = db
        .admin
        .query_one("SELECT count(*) FROM case_stage_revisions", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[test]
fn direct_sql_cannot_adopt_over_initial_registration_or_add_initial_after_adoption() {
    let Some(mut db) = Fixture::new() else { return };
    let case = CaseId::new();
    db.store()
        .register_penal(db.owner, case, creation("Initial"), db.at)
        .unwrap();
    let record = upload(&db, case, "support.pdf");
    let mut value = row_value(&db, &record);
    value["case_id"] = case.to_string().into();
    let signed = signed_value(&mut db.admin, &value);
    assert_eq!(
        db.runtime()
            .execute(INSERT, &[&signed])
            .unwrap_err()
            .code()
            .unwrap()
            .code(),
        "23514"
    );
    complete(&db);
    let record = upload(&db, db.case, "legacy.pdf");
    service(&db, db.owner, Role::Owner, FormatCheck(None))
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Investigation),
        )
        .unwrap();
    // Isolate the new cross-table guard from the existing initial-profile guard.
    db.admin
        .batch_execute(
            "ALTER TABLE case_initial_stage_registrations DISABLE TRIGGER case_initial_stage_valid",
        )
        .unwrap();
    let error=db.admin.execute("INSERT INTO case_initial_stage_registrations(case_id,stage) VALUES($1,'investigation')",&[&db.case.as_uuid()]).unwrap_err();
    assert_eq!(error.code().unwrap().code(), "23514");
    assert!(error
        .as_db_error()
        .unwrap()
        .message()
        .contains("cannot follow an adopted"));
}

#[test]
fn direct_sql_immutability_and_duplicate_support_decode_are_enforced() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    let workflow = service(&db, db.owner, Role::Owner, FormatCheck(None));
    workflow
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Intermediate),
        )
        .unwrap();
    let time = DeclaredStageTime::instant(db.at).unwrap();
    workflow
        .transition(
            "session",
            db.case,
            CaseStageRevision::FIRST,
            StageTransition::to_trial(
                time,
                reference(&record),
                time,
                StageCourt::new("Court").unwrap(),
                None,
                Some(reference(&record)),
                None,
            )
            .unwrap(),
        )
        .unwrap();
    for mutation in [
        "UPDATE case_stage_revisions SET reason='changed' WHERE revision=1",
        "DELETE FROM case_stage_revisions WHERE revision=1",
    ] {
        assert_eq!(
            db.admin
                .batch_execute(mutation)
                .unwrap_err()
                .code()
                .unwrap()
                .code(),
            "23514"
        );
    }
    let stages = store(&db);
    let before = snapshot(&mut db);
    db.admin.batch_execute("ALTER TABLE case_stage_revisions DISABLE TRIGGER USER; ALTER TABLE case_stage_revisions DROP CONSTRAINT case_stage_canonical; ALTER TABLE case_stage_revisions DROP CONSTRAINT case_stage_digest; UPDATE case_stage_revisions SET receipt_format='docx' WHERE revision=2; ALTER TABLE case_stage_revisions ENABLE TRIGGER USER").unwrap();
    assert!(matches!(
        stages.get(db.owner, db.case, db.at),
        Err(ApplicationError::StoredCaseStageInconsistent(_))
    ));
    let after = snapshot(&mut db);
    assert_eq!(after["audit"], before["audit"]);
}
