mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_dispatch_guard_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_dispatch_timeout_support;
mod deadline_profile_database_support;
mod procedural_fact_backend_support;

use application::{deadline_dispatch::DeadlineDispatchStore, ApplicationError, PortFailureKind};
use deadline_dispatch_guard_support as guards;
use deadline_dispatch_support as dispatch;
use deadline_dispatch_timeout_support as timeout;
use std::sync::{Arc, Mutex};

// Fixture schemas share the database-scoped audit lock.
static TIMEOUT_SCENARIO: Mutex<()> = Mutex::new(());

#[test]
fn audit_lock_timeout_preserves_progress_and_the_same_connection_recovers() {
    let _scenario = TIMEOUT_SCENARIO.lock().unwrap();
    let Some((mut db, changed, _)) = guards::setup(&[20, 40]) else {
        return;
    };
    let sequence = dispatch::event_sequence(&mut db, &changed);
    // Open first: startup validation also participates in the audited lock.
    let (store, pid) = timeout::open(&mut db);
    let before = guards::snapshot(&mut db);
    let mut held = db.admin.transaction().unwrap();
    held.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])
        .unwrap();
    let (receiver, worker) = timeout::start(Arc::clone(&store));
    let (completed_with_fault, outcome) = timeout::bounded_result(&mut db.control, pid, &receiver);
    // Cleanup precedes assertions even when a missing timeout trips the watchdog.
    held.rollback().unwrap();
    worker.join().unwrap();
    assert!(
        completed_with_fault,
        "dispatch required external cancellation while the audit lock remained held"
    );
    assert!(
        matches!(
            &outcome,
            Err(ApplicationError::ClassifiedPort {
                kind: PortFailureKind::Busy,
                ..
            })
        ),
        "{outcome:?}"
    );
    assert_eq!(guards::snapshot(&mut db), before);
    timeout::same_connection(&mut db, pid);
    let recovered = store.dispatch(timeout::request()).unwrap();
    assert_eq!(recovered.event.unwrap().sequence, sequence);
    assert_eq!((recovered.selected, recovered.inserted), (2, 2));
    assert!(recovered.completed_scan);
    timeout::same_connection(&mut db, pid);
    assert_eq!(
        dispatch::event_jobs(&mut db, sequence),
        vec![dispatch::id(20), dispatch::id(40)]
    );
}

#[test]
fn statement_timeout_rolls_back_jobs_cursor_and_audit_then_reuses_the_connection() {
    let _scenario = TIMEOUT_SCENARIO.lock().unwrap();
    let Some((mut db, changed, _)) = guards::setup(&[20, 40]) else {
        return;
    };
    let sequence = dispatch::event_sequence(&mut db, &changed);
    let (store, pid) = timeout::open(&mut db);
    let before = guards::snapshot(&mut db);
    // This extra callback preserves every real data guard and pauses after writes.
    timeout::slow_audit(&mut db, sequence);
    let (receiver, worker) = timeout::start(Arc::clone(&store));
    let (completed_with_fault, outcome) = timeout::bounded_result(&mut db.control, pid, &receiver);
    worker.join().unwrap();
    let reached = timeout::slow_audit_reached(&mut db);
    let after_failure = guards::snapshot(&mut db);
    timeout::remove_slow_audit(&mut db);
    assert!(
        completed_with_fault,
        "dispatch required external cancellation while its slow callback remained installed"
    );
    assert!(
        reached,
        "the injected delay was not reached after both job and cursor writes"
    );
    assert!(
        matches!(
            &outcome,
            Err(ApplicationError::ClassifiedPort {
                kind: PortFailureKind::Interrupted,
                ..
            })
        ),
        "{outcome:?}"
    );
    assert_eq!(after_failure, before);
    assert_eq!(guards::snapshot(&mut db), before);
    timeout::same_connection(&mut db, pid);
    let recovered = store.dispatch(timeout::request()).unwrap();
    assert_eq!(recovered.event.unwrap().sequence, sequence);
    assert_eq!((recovered.selected, recovered.inserted), (2, 2));
    assert!(recovered.completed_scan);
    timeout::same_connection(&mut db, pid);
    assert_eq!(
        dispatch::event_jobs(&mut db, sequence),
        vec![dispatch::id(20), dispatch::id(40)]
    );
}
