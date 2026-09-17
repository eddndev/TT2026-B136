mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_profile_database_support;
use application::{deadline_profiles::*, ApplicationError};
use deadline_profile_database_support::*;
use domain::identity::Role;

#[test]
fn profile_revisions_preserve_exact_definitions_receipts_and_terminal_retirement() {
    let Some(mut db) = Fixture::new() else { return };
    let service = service(&db, db.owner, Role::Owner);
    let collection = DeadlineProfileCollection::Global;
    let command = publish(None);
    let before = snapshot(&mut db);
    let draft = service
        .prepare("session", collection, command.clone())
        .unwrap();
    assert_eq!(snapshot(&mut db), before);
    let first = service
        .submit(
            "session",
            collection,
            command.clone(),
            draft.submission_digest,
        )
        .unwrap();
    assert_eq!(first.recorded_by.email, "owner@example.test");
    assert_eq!(first.receipt.operation_id, command.operation_id);
    assert_eq!(first.algorithm, DeadlineProfileAlgorithm::V1);
    let second = persist(&service, collection, replace(&first));
    let third = persist(&service, collection, retire(&second));
    assert_eq!(third.definition, second.definition);
    assert_eq!(third.definition_digest, second.definition_digest);
    assert_eq!(third.status, DeadlineProfileStatus::Retired);
    assert_eq!(
        service
            .get("session", collection, first.id, Some(first.revision))
            .unwrap(),
        first
    );
    assert_eq!(
        service.get("session", collection, first.id, None).unwrap(),
        third
    );
    let history = service
        .history(
            "session",
            collection,
            first.id,
            DeadlineProfileHistoryQuery::new(20, None).unwrap(),
        )
        .unwrap();
    assert_eq!(
        history.revisions,
        vec![(&third).into(), (&second).into(), (&first).into()]
    );
    let before = snapshot(&mut db);
    assert!(matches!(
        service.prepare("session", collection, replace(&third)),
        Err(ApplicationError::DeadlineProfile(
            DeadlineProfileError::Retired
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
}
#[test]
fn global_catalog_excludes_private_profiles_even_for_owner_and_filters_before_pagination() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let global = persist(&workflow, DeadlineProfileCollection::Global, publish(None));
    let private = persist(
        &workflow,
        DeadlineProfileCollection::ForCase(db.case),
        publish(Some(db.case)),
    );
    let listed = workflow
        .list("session", DeadlineProfileCollection::Global, query(1, None))
        .unwrap();
    assert_eq!(listed.profiles, vec![(&global).into()]);
    assert!(!listed.has_more);
    assert!(matches!(
        workflow.get(
            "session",
            DeadlineProfileCollection::Global,
            private.id,
            None
        ),
        Err(ApplicationError::DeadlineProfile(
            DeadlineProfileError::NotFound
        ))
    ));
    let assigned = db.user("litigator", true);
    let outsider = db.user("litigator", false);
    let adapter = store(&db);
    let scoped = adapter
        .list(
            assigned,
            DeadlineProfileCollection::ForCase(db.case),
            query(100, None),
            db.at,
        )
        .unwrap();
    assert_eq!(scoped.profiles.len(), 2);
    assert!(scoped.profiles.iter().any(|p| p.id == private.id));
    let before = snapshot(&mut db);
    assert!(matches!(
        adapter.list(
            outsider,
            DeadlineProfileCollection::ForCase(db.case),
            query(100, None),
            db.at
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        adapter.get(
            outsider,
            DeadlineProfileCollection::ForCase(db.case),
            global.id,
            None,
            db.at
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(snapshot(&mut db), before);
}
#[test]
fn inactive_or_nonowner_cannot_mutate_and_client_cannot_read_any_collection() {
    let Some(mut db) = Fixture::new() else { return };
    let collection = DeadlineProfileCollection::Global;
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(&workflow, collection, publish(None));
    for role in ["litigator", "paralegal", "client"] {
        let actor = db.user(role, true);
        let before = snapshot(&mut db);
        assert!(matches!(
            store(&db).prepare(actor, collection, &publish(None)),
            Err(ApplicationError::PermissionDenied)
        ));
        if role == "client" {
            assert!(matches!(
                store(&db).get(actor, collection, first.id, None, db.at),
                Err(ApplicationError::PermissionDenied)
            ));
        }
        assert_eq!(snapshot(&mut db), before);
    }
    let command = replace(&first);
    let draft = workflow
        .prepare("session", collection, command.clone())
        .unwrap();
    db.admin
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.submit("session", collection, command, draft.submission_digest),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(snapshot(&mut db), before);
}
#[test]
fn rejected_audit_rolls_back_profile_revision_root_and_change_event() {
    let Some(mut db) = Fixture::new() else { return };
    let collection = DeadlineProfileCollection::Global;
    let workflow = service(&db, db.owner, Role::Owner);
    let command = publish(None);
    let draft = workflow
        .prepare("session", collection, command.clone())
        .unwrap();
    db.admin.batch_execute("CREATE FUNCTION reject_profile_audit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.action LIKE 'deadline_profile.%' THEN RAISE EXCEPTION 'injected audit failure'; END IF; RETURN NEW; END; $$;
        CREATE TRIGGER reject_profile_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_profile_audit()").unwrap();
    let before = snapshot(&mut db);
    assert!(workflow
        .submit("session", collection, command, draft.submission_digest)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}
#[test]
fn every_profile_revision_emits_an_exact_global_or_case_change_event() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    for case in [None, Some(db.case)] {
        let collection = case
            .map(DeadlineProfileCollection::ForCase)
            .unwrap_or(DeadlineProfileCollection::Global);
        let first = persist(&workflow, collection, publish(case));
        let second = persist(&workflow, collection, replace(&first));
        let third = persist(&workflow, collection, retire(&second));
        for expected in [first, second, third] {
            let row=db.admin.query_one("SELECT case_id,operation_id,hearing_id FROM deadline_source_events WHERE source_kind='profile' AND source_id=$1 AND revision=$2",
                &[&expected.id.as_uuid(),&i64::from(expected.revision.get())]).unwrap();
            assert_eq!(
                row.get::<_, Option<uuid::Uuid>>(0),
                case.map(|id| id.as_uuid())
            );
            assert_eq!(
                row.get::<_, uuid::Uuid>(1),
                expected.receipt.operation_id.as_uuid()
            );
            assert!(row.get::<_, Option<uuid::Uuid>>(2).is_none());
        }
    }
}
