mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
use application::{
    cases::CaseRepository, documents::StageSupportReadLimits, procedural_facts::*, ApplicationError,
};
use domain::{
    cases::{CaseId, CaseMetadata},
    identity::Role,
};
use procedural_fact_backend_support::*;

#[test]
fn assigned_litigator_writes_paralegal_reads_and_client_has_no_fact_access() {
    let Some(mut db) = Fixture::new() else { return };
    let litigator = db.user("litigator", true);
    let first = persist(&service(&db, litigator, Role::Litigator), db.case, record());
    assert_eq!(first.snapshot.metadata().recorded_by.id, litigator);
    let paralegal = db.user("paralegal", true);
    let client = db.user("client", true);
    let read = service(&db, paralegal, Role::Paralegal);
    assert_eq!(
        read.get("session", db.case, first.snapshot.target(), None)
            .unwrap(),
        first
    );
    assert_eq!(
        read.list_resolutions(
            "session",
            db.case,
            ResolutionQuery::new(20, None, FactStatusFilter::All).unwrap()
        )
        .unwrap()
        .resolutions
        .len(),
        1
    );
    let adapter = store(&db);
    for actor in [paralegal, client] {
        let before = snapshot(&mut db);
        assert!(matches!(
            adapter.prepare(
                actor,
                db.case,
                &record(),
                &StageSupportReadLimits::standard()
            ),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(snapshot(&mut db), before);
    }
    let before = snapshot(&mut db);
    assert!(matches!(
        adapter.get(client, db.case, first.snapshot.target(), None, db.at),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        adapter.history(
            client,
            db.case,
            first.snapshot.target(),
            FactHistoryQuery::new(20, None).unwrap(),
            db.at
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        adapter.list_resolutions(
            client,
            db.case,
            ResolutionQuery::new(20, None, FactStatusFilter::All).unwrap(),
            db.at
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn unassigned_staff_and_disabled_actors_cannot_use_case_references() {
    let Some(mut db) = Fixture::new() else { return };
    let first = persist(&service(&db, db.owner, Role::Owner), db.case, record());
    let adapter = store(&db);
    for role in ["litigator", "paralegal"] {
        let actor = db.user(role, false);
        let before = snapshot(&mut db);
        assert!(matches!(
            adapter.get(actor, db.case, first.snapshot.target(), None, db.at),
            Err(ApplicationError::CaseNotFound)
        ));
        assert!(matches!(
            adapter.list_resolutions(
                actor,
                db.case,
                ResolutionQuery::new(20, None, FactStatusFilter::All).unwrap(),
                db.at
            ),
            Err(ApplicationError::CaseNotFound)
        ));
        if role == "litigator" {
            assert!(matches!(
                adapter.prepare(
                    actor,
                    db.case,
                    &record(),
                    &StageSupportReadLimits::standard()
                ),
                Err(ApplicationError::CaseNotFound)
            ));
        }
        assert_eq!(snapshot(&mut db), before);
    }
    db.admin
        .execute(
            "UPDATE users SET active=FALSE WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        adapter.get(db.owner, db.case, first.snapshot.target(), None, db.at),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        adapter.prepare(
            db.owner,
            db.case,
            &record(),
            &StageSupportReadLimits::standard()
        ),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn foreign_and_missing_parent_are_indistinguishable_and_never_create_a_notification() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let parent = persist(&workflow, db.case, record());
    let other = CaseId::new();
    db.store()
        .create_basic(
            db.owner,
            other,
            CaseMetadata::new("Other case", "REF-OTHER").unwrap(),
            db.at,
        )
        .unwrap();
    let before = snapshot(&mut db);
    for reference in [
        resolution_ref(&parent),
        FactResolutionRef {
            id: ResolutionId::new(),
            revision: FactRevision::initial(),
        },
    ] {
        assert!(matches!(
            workflow.prepare("session", other, notify(reference)),
            Err(ApplicationError::ProceduralFact(
                ProceduralFactError::ReferenceNotFound
            ))
        ));
    }
    assert!(matches!(
        workflow.get("session", other, parent.snapshot.target(), None),
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::NotFound
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
}
