mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod judicial_calendar_database_support;
mod judicial_calendar_interleaved_support;
use application::{judicial_calendars::*, ApplicationError};
use domain::identity::Role;
use judicial_calendar_database_support::*;
use judicial_calendar_interleaved_support::*;
use std::sync::{Arc, Barrier};

#[test]
fn two_owners_cannot_publish_two_successors_of_one_calendar_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let second_owner = db.user("owner", false);
    let first = persist(&service(&db, db.owner, Role::Owner), publish());
    let before = counts(&mut db);
    let barrier = Arc::new(Barrier::new(2));
    let workers = [(db.owner, replace(&first)), (second_owner, retire(&first))]
        .into_iter()
        .map(|(actor, command)| {
            let draft = service(&db, actor, Role::Owner)
                .prepare("session", command.clone())
                .unwrap();
            let barrier = barrier.clone();
            let workflow = hooked(&db, actor, move || {
                barrier.wait();
            });
            std::thread::spawn(move || workflow.submit("session", command, draft.submission_digest))
        })
        .collect::<Vec<_>>();
    let results = workers
        .into_iter()
        .map(|w| w.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(
                r,
                Err(ApplicationError::JudicialCalendar(
                    JudicialCalendarError::RevisionConflict
                ))
            ))
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(counts(&mut db), (before.0, before.1 + 1, before.2 + 1));
}
#[test]
fn one_operation_cannot_create_two_concurrent_roots() {
    let Some(mut db) = Fixture::new() else { return };
    let before = counts(&mut db);
    let barrier = Arc::new(Barrier::new(2));
    let operation = JudicialCalendarOperationId::new();
    let workers = (0..2)
        .map(|_| {
            let mut command = publish();
            command.operation_id = operation;
            let draft = service(&db, db.owner, Role::Owner)
                .prepare("session", command.clone())
                .unwrap();
            let barrier = barrier.clone();
            let workflow = hooked(&db, db.owner, move || {
                barrier.wait();
            });
            std::thread::spawn(move || workflow.submit("session", command, draft.submission_digest))
        })
        .collect::<Vec<_>>();
    let results = workers
        .into_iter()
        .map(|w| w.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(
                r,
                Err(ApplicationError::JudicialCalendar(
                    JudicialCalendarError::OperationConflict
                ))
            ))
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(counts(&mut db), (before.0 + 1, before.1 + 1, before.2 + 1));
}
#[test]
fn waiting_read_rechecks_active_staff_after_the_lock_even_with_repeatable_read_default() {
    let Some(mut db) = Fixture::new() else { return };
    let actor = db.user("litigator", false);
    let first = persist(&service(&db, db.owner, Role::Owner), publish());
    db.admin
        .batch_execute(&format!(
            "ALTER ROLE {} SET default_transaction_isolation='repeatable read'",
            db.role
        ))
        .unwrap();
    let reader = store(&db);
    let before = snapshot(&mut db);
    db.admin
        .batch_execute("SELECT pg_advisory_lock(280603412820)")
        .unwrap();
    let at = db.at;
    let worker = std::thread::spawn(move || reader.get(actor, first.id, None, at));
    let waiting = wait_for_lock(&mut db);
    db.admin
        .execute(
            "UPDATE users SET role='client' WHERE id=$1",
            &[&actor.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("SELECT pg_advisory_unlock(280603412820)")
        .unwrap();
    assert!(waiting, "reader did not wait for the common audit lock");
    assert!(matches!(
        worker.join().unwrap(),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(snapshot(&mut db), before);
}
