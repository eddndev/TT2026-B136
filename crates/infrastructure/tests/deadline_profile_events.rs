mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_profile_database_support;

use application::{cases::CaseRepository, deadline_profiles::*};
use deadline_profile_database_support::*;
use domain::{
    cases::{CaseId, CaseMetadata},
    identity::Role,
};
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use postgres::error::SqlState;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
struct Event {
    id: Uuid,
    revision: i64,
    case: Option<Uuid>,
    operation: Uuid,
}
fn insert(db: &mut Fixture, event: &Event) -> Result<u64, postgres::Error> {
    db.admin.execute("INSERT INTO deadline_source_events(source_kind,source_id,revision,case_id,operation_id) VALUES('profile',$1,$2,$3,$4)",
        &[&event.id,&event.revision,&event.case,&event.operation])
}
fn rejects(db: &mut Fixture, event: &Event) {
    db.admin
        .batch_execute("SAVEPOINT wrong_profile_event")
        .unwrap();
    let error = insert(db, event).unwrap_err();
    assert!(
        matches!(error.code(), Some(code) if *code == SqlState::CHECK_VIOLATION || *code == SqlState::FOREIGN_KEY_VIOLATION),
        "event must fail for its source, not a duplicate: {error:?}"
    );
    db.admin
        .batch_execute(
            "ROLLBACK TO SAVEPOINT wrong_profile_event; RELEASE SAVEPOINT wrong_profile_event",
        )
        .unwrap();
}

#[test]
fn direct_profile_events_require_exact_nullable_scope_operation_and_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let foreign = CaseId::new();
    db.store()
        .create_basic(
            db.owner,
            foreign,
            CaseMetadata::new("Other case", "OTHER").unwrap(),
            db.at,
        )
        .unwrap();
    let workflow = service(&db, db.owner, Role::Owner);
    for case in [None, Some(db.case)] {
        let collection = case
            .map(DeadlineProfileCollection::ForCase)
            .unwrap_or(DeadlineProfileCollection::Global);
        let profile = persist(&workflow, collection, publish(case));
        let expected = Event {
            id: profile.id.as_uuid(),
            revision: i64::from(profile.revision.get()),
            case: case.map(|id| id.as_uuid()),
            operation: profile.receipt.operation_id.as_uuid(),
        };
        let before = snapshot(&mut db);
        db.admin
            .batch_execute("BEGIN; ALTER TABLE deadline_source_events DISABLE TRIGGER USER")
            .unwrap();
        assert_eq!(db.admin.execute("DELETE FROM deadline_source_events WHERE source_kind='profile' AND source_id=$1 AND revision=$2",
            &[&expected.id,&expected.revision]).unwrap(), 1);
        db.admin
            .batch_execute("ALTER TABLE deadline_source_events ENABLE TRIGGER USER")
            .unwrap();
        for wrong_case in [
            Some(foreign.as_uuid()),
            if case.is_some() {
                None
            } else {
                Some(db.case.as_uuid())
            },
        ] {
            let mut wrong = expected.clone();
            wrong.case = wrong_case;
            rejects(&mut db, &wrong);
        }
        let mut wrong = expected.clone();
        wrong.operation = Uuid::new_v4();
        rejects(&mut db, &wrong);
        wrong = expected.clone();
        wrong.revision += 1;
        rejects(&mut db, &wrong);
        wrong = expected.clone();
        wrong.id = Uuid::new_v4();
        rejects(&mut db, &wrong);
        assert_eq!(insert(&mut db, &expected).unwrap(), 1);
        db.admin.batch_execute("ROLLBACK").unwrap();
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn startup_rejects_missing_profile_event_fk_emitter_generated_key_or_unsafe_grant() {
    for alteration in [
        "ALTER TABLE deadline_source_events DROP CONSTRAINT deadline_source_profile_revision",
        "ALTER TABLE deadline_source_events ALTER COLUMN profile_id DROP EXPRESSION",
        "ALTER TABLE deadline_profile_revisions DISABLE TRIGGER deadline_source_emit",
        "GRANT INSERT(profile_id) ON deadline_source_events TO {role}",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_ok());
        db.admin
            .batch_execute(&alteration.replace("{role}", &db.role))
            .unwrap();
        assert!(
            PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
            "startup must reject {alteration}"
        );
    }
}
