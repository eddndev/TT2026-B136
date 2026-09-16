mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
use application::{procedural_facts::*, ApplicationError};
use domain::identity::Role;
use procedural_fact_backend_support::*;

#[test]
fn audit_failure_rolls_back_both_fact_tables_and_does_not_reserve_an_operation() {
    for notification in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let workflow = service(&db, db.owner, Role::Owner);
        let command = if notification {
            let parent = persist(&workflow, db.case, record());
            notify(resolution_ref(&parent))
        } else {
            record()
        };
        let draft = workflow
            .prepare("session", db.case, command.clone())
            .unwrap();
        db.admin.batch_execute("CREATE FUNCTION reject_fact_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected fact audit failure'; END; $$; CREATE TRIGGER reject_fact_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_fact_audit()").unwrap();
        let before = snapshot(&mut db);
        assert!(matches!(
            workflow.submit("session", db.case, command.clone(), draft.submission_digest),
            Err(ApplicationError::Port(_))
        ));
        assert_eq!(snapshot(&mut db), before);
        db.admin
            .batch_execute(
                "DROP TRIGGER reject_fact_audit ON audit_events; DROP FUNCTION reject_fact_audit()",
            )
            .unwrap();
        let saved = workflow
            .submit("session", db.case, command.clone(), draft.submission_digest)
            .unwrap();
        assert_eq!(
            saved.snapshot.metadata().receipt.operation_id,
            command.operation_id()
        );
        assert_eq!(
            saved.snapshot.metadata().receipt.submission_digest,
            draft.submission_digest
        );
    }
}

#[test]
fn operation_uuid_is_unique_across_families_while_equal_root_uuid_bytes_are_distinct() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let resolution = persist(&workflow, db.case, record());
    let reference = resolution_ref(&resolution);
    let notice_id = NotificationId::from_uuid(reference.id.as_uuid());
    let reused = ProceduralFactCommand::Notification(
        NotificationCommand::new(
            resolution.snapshot.metadata().receipt.operation_id,
            notice_id,
            reference.id,
            FactChange::record(notification_values(reference, "Declared practice")),
        )
        .unwrap(),
    );
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.prepare("session", db.case, reused),
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::OperationConflict
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
    let notification = persist(
        &workflow,
        db.case,
        ProceduralFactCommand::Notification(
            NotificationCommand::new(
                FactOperationId::new(),
                notice_id,
                reference.id,
                FactChange::record(notification_values(reference, "Declared practice")),
            )
            .unwrap(),
        ),
    );
    assert_ne!(resolution.snapshot.target(), notification.snapshot.target());
    assert_eq!(
        workflow
            .get("session", db.case, resolution.snapshot.target(), None)
            .unwrap(),
        resolution
    );
    assert_eq!(
        workflow
            .get("session", db.case, notification.snapshot.target(), None)
            .unwrap(),
        notification
    );
    let reused = ProceduralFactCommand::Resolution(ResolutionCommand::new(
        notification.snapshot.metadata().receipt.operation_id,
        ResolutionId::new(),
        FactChange::record(values("Another declaration")),
    ));
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.prepare("session", db.case, reused),
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::OperationConflict
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
}
