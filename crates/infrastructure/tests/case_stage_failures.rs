mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;

use application::case_stages::*;
use application::ApplicationError;
use case_stage_database_support::*;
use domain::identity::Role;

#[test]
fn insert_audit_and_deferred_commit_failures_preserve_complete_history_and_audit() {
    for failure in [
        "CREATE FUNCTION reject_stage() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected insert failure'; END $$; CREATE TRIGGER reject_stage BEFORE INSERT ON case_stage_revisions FOR EACH ROW EXECUTE FUNCTION reject_stage()",
        "ALTER TABLE audit_events ADD CONSTRAINT reject_stage CHECK(action<>'case.stage_adopted')",
        "CREATE FUNCTION reject_stage() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred failure'; END $$; CREATE CONSTRAINT TRIGGER reject_stage AFTER INSERT ON case_stage_revisions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_stage()",
    ] {
        let Some(mut db)=Fixture::new() else {return};
        complete(&db);
        let record=upload(&db,db.case,"support.pdf");
        let before=snapshot(&mut db);
        let url=db.admin_url.clone();
        let format=FormatCheck(Some(Box::new(move || {postgres::Client::connect(&url,postgres::NoTls).unwrap().batch_execute(failure).unwrap();})));
        let result=service(&db,db.owner,Role::Owner,format).adopt("session",db.case,CaseStageExpectation::Unregistered,adoption(&db,&record,CaseStage::Investigation));
        assert!(matches!(result,Err(ApplicationError::Port(_))),"{result:?}");
        assert_eq!(snapshot(&mut db),before);
    }
}

#[test]
fn reads_do_not_expose_results_when_audit_insertion_fails() {
    for history in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let stages = store(&db);
        let before = snapshot(&mut db);
        db.admin.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_stage CHECK(action NOT IN ('case.stage_read','case.stage_history_read'))").unwrap();
        let result = if history {
            stages
                .history(
                    db.owner,
                    db.case,
                    CaseStageQuery::new(1, None).unwrap(),
                    db.at,
                )
                .map(|_| ())
        } else {
            stages.get(db.owner, db.case, db.at).map(|_| ())
        };
        assert!(matches!(result, Err(ApplicationError::Port(_))));
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn malformed_encrypted_support_is_not_parsed_or_committed() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    db.admin
        .batch_execute("ALTER TABLE documents DISABLE TRIGGER USER")
        .unwrap();
    db.admin.execute("UPDATE documents SET vault=set_byte(vault,octet_length(vault)-1,get_byte(vault,octet_length(vault)-1)#1) WHERE id=$1",&[&record.id.as_uuid()]).unwrap();
    db.admin
        .batch_execute("ALTER TABLE documents ENABLE TRIGGER USER")
        .unwrap();
    let before = snapshot(&mut db);
    let format = FormatCheck(Some(Box::new(|| {
        panic!("parser must not receive unauthenticated plaintext")
    })));
    assert!(service(&db, db.owner, Role::Owner, format)
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Investigation)
        )
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn oversized_legacy_vault_and_evidence_are_rejected_before_decoding_or_crypto() {
    for field in ["vault", "evidence"] {
        let Some(mut db) = Fixture::new() else { return };
        complete(&db);
        let record = upload(&db, db.case, "support.pdf");
        let stages = store(&db);
        db.admin
            .batch_execute("ALTER TABLE documents DISABLE TRIGGER USER")
            .unwrap();
        let sql = if field == "vault" {
            "UPDATE documents SET vault=vault||decode(repeat('aa',16777313),'hex') WHERE id=$1"
        } else {
            "UPDATE documents SET evidence=jsonb_build_object('invalid',repeat('x',1048577)) WHERE id=$1"
        };
        db.admin.execute(sql, &[&record.id.as_uuid()]).unwrap();
        db.admin
            .batch_execute("ALTER TABLE documents ENABLE TRIGGER USER")
            .unwrap();
        let before = snapshot(&mut db);
        let change = CaseStageChange::Adopt(adoption(&db, &record, CaseStage::Investigation));
        let result = stages.prepare(
            db.owner,
            db.case,
            CaseStageExpectation::Unregistered,
            &change,
            &StageSupportReadLimits::standard(),
        );
        assert!(
            matches!(result, Err(ApplicationError::StageSupportTooLarge)),
            "{field}: {result:?}"
        );
        assert_eq!(snapshot(&mut db), before);
    }
}
