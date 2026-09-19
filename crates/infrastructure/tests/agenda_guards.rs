mod agenda_backend_support;
mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod hearing_database_support;
mod procedural_fact_backend_support;

use agenda_backend_support::*;
use application::{agenda::*, deadlines::DeadlineError, ApplicationError};

#[test]
fn altered_candidate_due_projection_fails_without_an_agenda_audit() {
    let Some(mut db) = fixture() else { return };
    let (_, base) = accepted(&db, 1);
    let store = store(&db);
    db.admin
        .batch_execute("SET session_replication_role='replica'")
        .unwrap();
    db.admin
        .execute(
            "UPDATE case_deadline_revisions SET due_at_seconds=due_at_seconds+1
            WHERE deadline_id=$1",
            &[&base.id.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("SET session_replication_role='origin'")
        .unwrap();
    let before = unchanged_resources(&mut db);
    assert!(matches!(
        store.list(db.owner, query(20, AgendaKind::All, None)),
        Err(ApplicationError::Deadline(
            DeadlineError::StoredInconsistent(_)
        ))
    ));
    assert_eq!(unchanged_resources(&mut db), before);
    assert_eq!(audits(&mut db), 0);
}

#[test]
fn failed_agenda_audit_returns_an_error_and_rolls_back_the_page() {
    let Some(mut db) = fixture() else { return };
    accepted(&db, 1);
    let store = store(&db);
    db.admin
        .batch_execute(
            "CREATE FUNCTION reject_agenda_audit() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN IF NEW.action='agenda.read' THEN
                RAISE EXCEPTION 'injected agenda audit failure';
            END IF; RETURN NEW; END $$;
        CREATE TRIGGER reject_agenda_audit BEFORE INSERT ON audit_events
            FOR EACH ROW EXECUTE FUNCTION reject_agenda_audit()",
        )
        .unwrap();
    let before = unchanged_resources(&mut db);
    assert!(matches!(
        store.list(db.owner, query(20, AgendaKind::All, None)),
        Err(ApplicationError::Port(message)) if message.starts_with("audit database:")
    ));
    assert_eq!(unchanged_resources(&mut db), before);
    assert_eq!(audits(&mut db), 0);
}
