mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;

use application::case_stages::*;
use application::cases::*;
use application::documents::{CaseDocumentStore, DocumentRecord};
use application::ApplicationError;
use case_stage_database_support::*;
use domain::identity::Role;
use infrastructure::PostgresCaseDocumentStore;

#[test]
fn append_during_validation_keeps_exact_original_support_and_captures_current_administration() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    let next = processor()
        .prepare_version(
            record.id,
            record.version.next().unwrap(),
            "next.pdf",
            b"different content",
        )
        .unwrap();
    let documents = PostgresCaseDocumentStore::open(&db.runtime_url).unwrap();
    let repository = db.store();
    let (actor, case, at, version) = (db.owner, db.case, db.at, record.version);
    let format = FormatCheck(Some(Box::new(move || {
        documents
            .append(actor, case, version, next.clone(), at)
            .unwrap();
        repository
            .replace_administration(
                actor,
                case,
                CaseRevisionExpectation::new(1),
                creation("Current administration")
                    .into_values()
                    .editable()
                    .clone(),
                at,
            )
            .unwrap();
    })));
    let detail = service(&db, db.owner, Role::Owner, format)
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Investigation),
        )
        .unwrap();
    let CaseStageEntry::Changed(snapshot) = detail.current.entry().unwrap() else {
        panic!("expected stage")
    };
    assert_eq!(
        snapshot.supports[0].reference,
        reference(&record).reference()
    );
    assert_eq!(snapshot.supports[0].name, "support.pdf");
    assert_eq!(snapshot.administration_revision.get(), 2);
    assert_eq!(store(&db).get(db.owner, db.case, db.at).unwrap(), detail);
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM documents WHERE id=$1",
            &[&record.id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 2);
}

#[test]
fn real_seal_during_validation_conflicts_but_seal_after_commit_preserves_stage_history() {
    for concurrent in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        complete(&db);
        let record = upload(&db, db.case, "support.pdf");
        let sealed = processor().seal(&record).unwrap();
        let documents = PostgresCaseDocumentStore::open(&db.runtime_url).unwrap();
        let (actor, case, at) = (db.owner, db.case, db.at);
        let copy = sealed.clone();
        let format = FormatCheck(if concurrent {
            Some(Box::new(move || {
                documents.seal(actor, case, copy.clone(), at).unwrap()
            }))
        } else {
            None
        });
        let result = service(&db, db.owner, Role::Owner, format).adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Investigation),
        );
        if concurrent {
            assert!(matches!(result, Err(ApplicationError::StageSupportChanged)));
            let count: i64 = db
                .admin
                .query_one("SELECT count(*) FROM case_stage_revisions", &[])
                .unwrap()
                .get(0);
            assert_eq!(count, 0);
        } else {
            let detail = result.unwrap();
            PostgresCaseDocumentStore::open(&db.runtime_url)
                .unwrap()
                .seal(actor, case, sealed, at)
                .unwrap();
            assert_eq!(store(&db).get(actor, case, at).unwrap(), detail);
        }
    }
}

#[test]
fn trial_preserves_two_exact_supports_date_precision_and_optional_receipt() {
    let Some(db) = Fixture::new() else { return };
    complete(&db);
    let order = upload(&db, db.case, "order.pdf");
    let receipt = upload(&db, db.case, "receipt.pdf");
    let workflow = service(&db, db.owner, Role::Owner, FormatCheck(None));
    workflow
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &order, CaseStage::Intermediate),
        )
        .unwrap();
    let day = DeclaredStageTime::date(
        time::macros::date!(2024 - 12 - 31),
        time::macros::offset!(-6),
    )
    .unwrap();
    let value = StageTransition::to_trial(
        day,
        reference(&order),
        DeclaredStageTime::instant(db.at).unwrap(),
        StageCourt::new("Court").unwrap(),
        Some(StageReceiptReference::new("Receipt-1").unwrap()),
        Some(reference(&receipt)),
        Some(StageNote::new("First\r\nSecond").unwrap()),
    )
    .unwrap();
    let result = workflow
        .transition("session", db.case, CaseStageRevision::FIRST, value.clone())
        .unwrap();
    let CaseStageEntry::Changed(snapshot) = result.current.entry().unwrap() else {
        panic!("expected stage")
    };
    assert_eq!(snapshot.supports.len(), 2);
    assert_eq!(snapshot.values, CaseStageChange::Transition(value));
    assert_eq!(store(&db).get(db.owner, db.case, db.at).unwrap(), result);
}

#[test]
fn historical_content_version_seven_retains_its_aad_and_exact_identity() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let first = processor()
        .prepare("legacy.pdf", b"legacy content")
        .unwrap();
    let version = domain::crypto::DocumentVersion::new(7).unwrap();
    let record: DocumentRecord = processor()
        .prepare_version(first.id, version, "legacy.pdf", b"legacy content")
        .unwrap();
    let mut tx = db.admin.transaction().unwrap();
    tx.execute(
        "INSERT INTO document_series(id,case_id,first_available_version) VALUES($1,$2,7)",
        &[&record.id.as_uuid(), &db.case.as_uuid()],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO documents(id,case_id,version,name,digest,vault) VALUES($1,$2,7,$3,$4,$5)",
        &[
            &record.id.as_uuid(),
            &db.case.as_uuid(),
            &record.name,
            &record.digest.as_bytes().as_slice(),
            &record.vault,
        ],
    )
    .unwrap();
    tx.commit().unwrap();
    let result = service(&db, db.owner, Role::Owner, FormatCheck(None))
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Trial),
        )
        .unwrap();
    let CaseStageEntry::Changed(snapshot) = result.current.entry().unwrap() else {
        panic!("expected stage")
    };
    assert_eq!(snapshot.supports[0].reference.version.get(), 7);
}
