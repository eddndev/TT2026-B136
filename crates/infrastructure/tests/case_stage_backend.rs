mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;

use application::case_stages::*;
use application::cases::CaseRepository;
use application::ApplicationError;
use case_stage_database_support::*;
use domain::cases::CaseId;
use domain::identity::Role;

#[test]
fn complete_legacy_adoption_captures_exact_content_and_current_author_then_reads_history() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    let stages = store(&db);
    assert_eq!(
        stages.get(db.owner, db.case, db.at).unwrap().current,
        CurrentCaseStage::Unregistered
    );
    let result = service(&db, db.owner, Role::Owner, FormatCheck(None))
        .adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Intermediate),
        )
        .unwrap();
    assert_eq!(result.current.revision(), Some(CaseStageRevision::FIRST));
    let CaseStageEntry::Changed(saved) = result.current.entry().unwrap() else {
        panic!("expected change")
    };
    assert_eq!(saved.from_stage, None);
    assert_eq!(saved.administration_revision.get(), 1);
    assert_eq!(saved.recorded_by.email, "owner@example.test");
    assert_eq!(saved.supports[0].reference, reference(&record).reference());
    assert_eq!(saved.supports[0].digest, record.digest);
    assert_eq!(saved.supports[0].name, record.name);
    db.admin
        .execute(
            "UPDATE users SET email='renamed@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let history = stages
        .history(
            db.owner,
            db.case,
            CaseStageQuery::new(1, None).unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(
        history.entries,
        vec![result.current.entry().unwrap().clone()]
    );
    assert!(!history.has_more);
    assert_eq!(history.next_before_revision, None);
    assert!(db
        .store()
        .get_administration(db.owner, db.case, db.at)
        .unwrap()
        .initial_stage
        .is_none());
}

#[test]
fn initial_registration_is_included_before_pagination_without_being_copied() {
    let Some(mut db) = Fixture::new() else { return };
    let case = CaseId::new();
    let original = db
        .store()
        .register_penal(db.owner, case, creation("Penal"), db.at)
        .unwrap()
        .initial_stage
        .unwrap();
    let record = upload(&db, case, "accusation.pdf");
    let stages = store(&db);
    assert_eq!(
        stages.get(db.owner, case, db.at).unwrap().current.entry(),
        Some(&CaseStageEntry::Initial(original.clone()))
    );
    let value = StageTransition::to_intermediate(
        DeclaredStageTime::instant(db.at).unwrap(),
        reference(&record),
        None,
    );
    service(&db, db.owner, Role::Owner, FormatCheck(None))
        .transition("session", case, CaseStageRevision::FIRST, value)
        .unwrap();
    let page = stages
        .history(db.owner, case, CaseStageQuery::new(1, None).unwrap(), db.at)
        .unwrap();
    assert_eq!(page.entries[0].stage_revision().get(), 2);
    assert!(page.has_more);
    assert_eq!(page.next_before_revision.unwrap().get(), 2);
    let last = stages
        .history(
            db.owner,
            case,
            CaseStageQuery::new(1, Some(2)).unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(last.entries, vec![CaseStageEntry::Initial(original)]);
    assert!(!last.has_more);
    let rows: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_stage_revisions WHERE case_id=$1",
            &[&case.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(rows, 1);
}

#[test]
fn pending_profiles_and_wrong_case_supports_fail_without_success_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "support.pdf");
    let before = snapshot(&mut db);
    let result = service(&db, db.owner, Role::Owner, FormatCheck(None)).adopt(
        "session",
        db.case,
        CaseStageExpectation::Unregistered,
        adoption(&db, &record, CaseStage::Investigation),
    );
    assert!(matches!(
        result,
        Err(ApplicationError::CaseStageProfileIncomplete)
    ));
    assert_eq!(snapshot(&mut db), before);
    complete(&db);
    let other = CaseId::new();
    db.store()
        .register_penal(db.owner, other, creation("Other"), db.at)
        .unwrap();
    let stages = store(&db);
    let change = CaseStageChange::Adopt(adoption(&db, &record, CaseStage::Investigation));
    assert!(matches!(
        stages.prepare(
            db.owner,
            other,
            CaseStageExpectation::Unregistered,
            &change,
            &StageSupportReadLimits::standard()
        ),
        Err(ApplicationError::DocumentNotFound(_))
    ));
}
