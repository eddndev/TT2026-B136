mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod typed_participant_service_support;
use application::{typed_participants::*, ApplicationError};
use std::sync::{Arc, Mutex};
use typed_participant_service_support::*;

#[test]
fn insert_review_audit_and_deferred_failures_rollback_the_compound_change() {
    for failure in [
        "CREATE FUNCTION reject_typed() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected typed insert'; END $$;CREATE TRIGGER reject_typed BEFORE INSERT ON case_participant_typed_revisions FOR EACH ROW EXECUTE FUNCTION reject_typed()",
        "ALTER TABLE participant_identity_reviews ADD CONSTRAINT reject_typed CHECK(FALSE)",
        "ALTER TABLE audit_events ADD CONSTRAINT reject_typed CHECK(action<>'participant.typed_created')",
        "CREATE FUNCTION reject_typed() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected deferred failure'; END $$;CREATE CONSTRAINT TRIGGER reject_typed AFTER INSERT ON case_participant_typed_revisions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_typed()",
    ]{
        let Some(mut db)=Fixture::new() else{return};let record=upload(&db,db.case,"support.pdf");
        let request=reviewed(service(&db,FormatCheck(None)).review_participant("session",db.case,proposal(&record)).unwrap());
        let before=Arc::new(Mutex::new(None));let captured=before.clone();let url=db.admin_url.clone();
        let format=FormatCheck(Some(Box::new(move||{let mut client=postgres::Client::connect(&url,postgres::NoTls).unwrap();client.batch_execute(failure).unwrap();*captured.lock().unwrap()=Some(snapshot_client(&mut client));})));
        let result=service(&db,format).submit_participant("session",db.case,ParticipantSubmission{prepared:request,signature:None});
        assert!(matches!(result,Err(ApplicationError::Port(_))),"{result:?}");
        assert_eq!(snapshot(&mut db),before.lock().unwrap().clone().unwrap());
    }
}
#[test]
fn concurrent_seal_changes_prepared_snapshot_and_prevents_typing() {
    let Some(mut db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "support.pdf");
    let request = reviewed(
        service(&db, FormatCheck(None))
            .review_participant("session", db.case, proposal(&record))
            .unwrap(),
    );
    let sealed = processor().seal(&record).unwrap();
    let runtime = db.runtime_url.clone();
    let (case, actor, at) = (db.case, db.owner, db.at);
    let format = FormatCheck(Some(Box::new(move || {
        use application::documents::CaseDocumentStore;
        infrastructure::PostgresCaseDocumentStore::open(&runtime)
            .unwrap()
            .seal(actor, case, sealed.clone(), at)
            .unwrap();
    })));
    let result = service(&db, format).submit_participant(
        "session",
        db.case,
        ParticipantSubmission {
            prepared: request,
            signature: None,
        },
    );
    assert!(
        matches!(result, Err(ApplicationError::ParticipantSupportChanged)),
        "{result:?}"
    );
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_subjects", &[])
            .unwrap()
            .get::<_, i64>(0),
        0
    );
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM audit_events WHERE action='participant.typed_created'",
                &[]
            )
            .unwrap()
            .get::<_, i64>(0),
        0
    );
}
#[test]
fn oversized_legacy_content_is_rejected_before_decryption_or_parser() {
    for field in ["vault", "evidence"] {
        let Some(mut db) = Fixture::new() else { return };
        let record = upload(&db, db.case, "support.pdf");
        let workflow = service(
            &db,
            FormatCheck(Some(Box::new(|| {
                panic!("oversized support reached parser")
            }))),
        );
        let request = reviewed(
            workflow
                .review_participant("session", db.case, proposal(&record))
                .unwrap(),
        );
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
        assert!(matches!(
            workflow.submit_participant(
                "session",
                db.case,
                ParticipantSubmission {
                    prepared: request,
                    signature: None
                }
            ),
            Err(ApplicationError::StageSupportTooLarge)
        ));
        assert_eq!(snapshot(&mut db), before);
    }
}
