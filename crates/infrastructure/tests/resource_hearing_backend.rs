mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod procedural_fact_backend_support;
mod procedural_resource_support;
mod resource_activity_support;
mod resource_hearing_database_support;
use application::{resource_activities::*, resource_hearings::*, ApplicationError};
use domain::{crypto::Sha256Digest, identity::Role};
use resource_hearing_database_support::*;

#[test]
fn exact_creation_survives_reopened_connection_and_unlinked_origin() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let result = submit(&db, command.clone());
    assert_eq!(
        result.association.sources.target,
        ResourceActivityTargetDetail::ResourceHearing(Box::new(result.hearing.clone()))
    );
    resource_activity_support::persist(
        &resource_activity_support::service(&db, db.owner, Role::Owner),
        db.case,
        captures.resource.id,
        resource_activity_support::unlink(&result.association, captures.head.revision),
    );
    captures.archive(&db);
    let before = atomic_rows(&mut db);
    let reopened = service(&db, db.owner, Role::Owner);
    let replay = reopened
        .submit(
            "session",
            db.case,
            captures.resource.id,
            command,
            result.origin.submission_digest,
        )
        .unwrap();
    assert_eq!(replay, result);
    assert_eq!(atomic_rows(&mut db), before);
}

#[test]
fn association_or_origin_audit_failure_rolls_back_every_mutation() {
    let Some(mut db) = Fixture::new() else { return };
    let (_, command) = setup(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let draft = workflow
        .prepare("session", db.case, command.resource.id, command.clone())
        .unwrap();
    for action in ["resource_activity.link", "resource_hearing.registered"] {
        db.admin.batch_execute(&format!("CREATE FUNCTION reject_hearing_commit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected atomic mutation failure'; END; $$; CREATE TRIGGER reject_hearing_commit BEFORE INSERT ON audit_events FOR EACH ROW WHEN (NEW.action='{action}') EXECUTE FUNCTION reject_hearing_commit()")).unwrap();
        let before = atomic_rows(&mut db);
        assert!(matches!(
            workflow.submit(
                "session",
                db.case,
                command.resource.id,
                command.clone(),
                draft.submission_digest
            ),
            Err(ApplicationError::Port(_))
        ));
        assert_eq!(atomic_rows(&mut db), before);
        db.admin.batch_execute("DROP TRIGGER reject_hearing_commit ON audit_events; DROP FUNCTION reject_hearing_commit()").unwrap();
    }
    workflow
        .submit(
            "session",
            db.case,
            command.resource.id,
            command,
            draft.submission_digest,
        )
        .unwrap();
}

#[test]
fn changed_intent_or_head_cannot_create_an_orphan() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let draft = workflow
        .prepare("session", db.case, command.resource.id, command.clone())
        .unwrap();
    let before = atomic_rows(&mut db);
    assert!(workflow
        .submit(
            "session",
            db.case,
            command.resource.id,
            command.clone(),
            Sha256Digest::from_array([9; 32])
        )
        .is_err());
    assert_eq!(atomic_rows(&mut db), before);
    captures.archive(&db);
    let before = atomic_rows(&mut db);
    assert!(workflow
        .submit(
            "session",
            db.case,
            command.resource.id,
            command,
            draft.submission_digest
        )
        .is_err());
    assert_eq!(atomic_rows(&mut db), before);
}

#[test]
fn replay_requires_current_membership_before_returning_history() {
    let Some(mut db) = Fixture::new() else { return };
    let (_, command) = setup(&mut db);
    let user = db.user("litigator", true);
    let workflow = service(&db, user, Role::Litigator);
    let draft = workflow
        .prepare("session", db.case, command.resource.id, command.clone())
        .unwrap();
    workflow
        .submit(
            "session",
            db.case,
            command.resource.id,
            command.clone(),
            draft.submission_digest,
        )
        .unwrap();
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE user_id=$1",
            &[&user.as_uuid()],
        )
        .unwrap();
    let before = atomic_rows(&mut db);
    assert!(matches!(
        workflow.submit(
            "session",
            db.case,
            command.resource.id,
            command,
            draft.submission_digest
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(atomic_rows(&mut db), before);
}

#[test]
fn raced_exact_preparations_return_one_durable_creation() {
    let Some(mut db) = Fixture::new() else { return };
    let (_, command) = setup(&mut db);
    let a = store(&db);
    let b = store(&db);
    let actor = application::identity::Principal {
        id: db.owner,
        email: "owner@example.test".into(),
        role: Role::Owner,
    };
    let prepare = |backend: &infrastructure::PostgresResourceHearingStore| {
        let ResourceHearingPreparation::Ready(material) = backend
            .prepare(db.owner, db.case, command.resource.id, &command)
            .unwrap()
        else {
            panic!("new operation")
        };
        prepare_resource_hearing_change(
            std::sync::Arc::new(infrastructure::RingSha256Hasher),
            &actor,
            db.case,
            command.resource.id,
            command.clone(),
            *material,
        )
        .unwrap()
    };
    let p = prepare(a.as_ref());
    let q = prepare(b.as_ref());
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let owner = db.owner;
    let case = db.case;
    let resource = command.resource.id;
    let (first, second) = std::thread::scope(|scope| {
        let one = barrier.clone();
        let first = scope.spawn(move || {
            one.wait();
            a.commit(owner, case, resource, p).unwrap()
        });
        let second = scope.spawn(move || {
            barrier.wait();
            b.commit(owner, case, resource, q).unwrap()
        });
        (first.join().unwrap(), second.join().unwrap())
    });
    assert_eq!(first, second);
    let row = db
        .admin
        .query_one(
            "SELECT (SELECT count(*) FROM case_resource_hearing_revisions),
        (SELECT count(*) FROM case_resource_activity_association_revisions),
        (SELECT count(*) FROM audit_events WHERE action='resource_hearing.registered')",
            &[],
        )
        .unwrap();
    assert_eq!(
        (
            row.get::<_, i64>(0),
            row.get::<_, i64>(1),
            row.get::<_, i64>(2)
        ),
        (1, 1, 1)
    );
}

#[test]
fn missing_creation_marker_is_not_adopted_from_an_existing_hearing_and_link() {
    let Some(mut db) = Fixture::new() else { return };
    let (_, command) = setup(&mut db);
    let result = submit(&db, command.clone());
    let workflow = service(&db, db.owner, Role::Owner);
    db.admin
        .batch_execute(
            "ALTER TABLE audit_events DISABLE TRIGGER USER;
        DELETE FROM audit_events WHERE action='resource_hearing.registered'",
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE audit_events ENABLE TRIGGER USER")
        .unwrap();
    let before = atomic_rows(&mut db);
    assert!(workflow
        .submit(
            "session",
            db.case,
            command.resource.id,
            command,
            result.origin.submission_digest
        )
        .is_err());
    assert_eq!(atomic_rows(&mut db), before);
    assert!(infrastructure::PostgresResourceHearingStore::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
        std::sync::Arc::new(case_stage_database_support::FixedClock(db.at))
    )
    .is_err());
}

#[test]
fn changed_projection_fails_replay_and_restored_inventory_validation() {
    let Some(mut db) = Fixture::new() else { return };
    let (_, command) = setup(&mut db);
    let result = submit(&db, command.clone());
    db.migrate();
    assert_eq!(
        service(&db, db.owner, Role::Owner)
            .submit(
                "session",
                db.case,
                command.resource.id,
                command.clone(),
                result.origin.submission_digest
            )
            .unwrap(),
        result
    );
    let workflow = service(&db, db.owner, Role::Owner);
    db.admin.batch_execute("ALTER TABLE case_resource_hearing_revisions DISABLE TRIGGER USER;
        UPDATE case_resource_hearing_revisions SET values_view=jsonb_set(values_view,'{venue}','\"Altered court\"');
        ALTER TABLE case_resource_hearing_revisions ENABLE TRIGGER USER").unwrap();
    assert!(workflow
        .submit(
            "session",
            db.case,
            command.resource.id,
            command,
            result.origin.submission_digest
        )
        .is_err());
    assert!(infrastructure::PostgresResourceHearingStore::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
        std::sync::Arc::new(case_stage_database_support::FixedClock(db.at))
    )
    .is_err());
}

#[test]
fn surviving_origin_rejects_partial_restore_and_duplicate_creation() {
    let Some(mut db) = Fixture::new() else { return };
    let (_, command) = setup(&mut db);
    let result = submit(&db, command.clone());
    let workflow = service(&db, db.owner, Role::Owner);
    db.admin
        .batch_execute(
            "SET session_replication_role=replica;
        DELETE FROM case_resource_activity_association_revisions;
        DELETE FROM case_resource_hearing_revisions;
        DELETE FROM case_resource_activity_associations;
        DELETE FROM case_resource_hearings;
        SET session_replication_role=origin",
        )
        .unwrap();
    assert!(
        infrastructure::PostgresResourceHearingStore::open(
            &db.runtime_url,
            std::sync::Arc::new(infrastructure::RingSha256Hasher),
            std::sync::Arc::new(case_stage_database_support::FixedClock(db.at))
        )
        .is_err(),
        "restored origin must still identify its hearing and initial association"
    );
    let before = atomic_rows(&mut db);
    assert!(
        workflow
            .submit(
                "session",
                db.case,
                command.resource.id,
                command,
                result.origin.submission_digest
            )
            .is_err(),
        "surviving origin must not be overwritten by a new creation"
    );
    assert_eq!(atomic_rows(&mut db), before);
}

#[path = "resource_hearing_database_support/participants.rs"]
mod participant_capture;
