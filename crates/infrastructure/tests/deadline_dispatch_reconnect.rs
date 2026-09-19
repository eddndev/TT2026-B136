mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_dispatch_guard_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
#[allow(dead_code)]
mod deadline_dispatch_timeout_support;
mod deadline_profile_database_support;
mod procedural_fact_backend_support;

use application::{deadline_dispatch::*, ApplicationError, PortFailureKind};
use deadline_backend_support as dl;
use deadline_dispatch_guard_support as guards;
use deadline_dispatch_support as dispatch;
use deadline_dispatch_timeout_support as timeout;
use serde_json::Value;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

// Fixture schemas share the database-scoped audit lock.
static RECONNECT_SCENARIO: Mutex<()> = Mutex::new(());

fn request(limit: u32) -> DeadlineDispatchRequest {
    DeadlineDispatchRequest {
        stream: DeadlineDispatchStream::Events,
        limit: DeadlineDispatchLimit::new(limit).unwrap(),
    }
}

fn eventually(
    store: &dyn DeadlineDispatchStore,
    request: DeadlineDispatchRequest,
) -> timeout::Outcome {
    // A closed transport may be observed on the first request after termination.
    // Every subsequent request still uses this same store and its saved URL.
    for attempt in 0..3 {
        let result = store.dispatch(request);
        if attempt == 2
            || !matches!(
                &result,
                Err(ApplicationError::ClassifiedPort {
                    kind: PortFailureKind::Unavailable,
                    ..
                })
            )
        {
            return result;
        }
    }
    unreachable!("the finite reconnect attempts always return an outcome")
}

fn application_name(db: &mut dl::Fixture, pid: i32) -> String {
    db.control
        .query_one(
            "SELECT application_name FROM pg_stat_activity WHERE pid=$1 AND usename=$2",
            &[&pid, &db.role],
        )
        .unwrap()
        .get(0)
}

fn terminate(db: &mut dl::Fixture, pid: i32) {
    assert!(db
        .control
        .query_one("SELECT pg_terminate_backend($1)", &[&pid])
        .unwrap()
        .get::<_, bool>(0));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let gone: bool = db
            .control
            .query_one(
                "SELECT NOT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1)",
                &[&pid],
            )
            .unwrap()
            .get(0);
        if gone {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "terminated backend remained live"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn replacement_pid(db: &mut dl::Fixture, application: &str, old: i32) -> i32 {
    let rows = db
        .control
        .query(
            "SELECT pid FROM pg_stat_activity WHERE usename=$1 AND application_name=$2",
            &[&db.role, &application],
        )
        .unwrap();
    assert_eq!(rows.len(), 1, "the named dispatcher needs one live backend");
    let pid = rows[0].get(0);
    assert_ne!(pid, old, "a terminated backend cannot be reused");
    pid
}

fn advanced_audits(db: &mut dl::Fixture) -> i64 {
    db.admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='deadline.dispatch_advanced'",
            &[],
        )
        .unwrap()
        .get(0)
}

#[test]
fn same_dispatcher_reconnects_at_its_committed_position_without_duplicate_work_or_audit() {
    let _scenario = RECONNECT_SCENARIO
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let Some((mut db, changed, _)) = guards::setup(&[20, 40, 60]) else {
        return;
    };
    let sequence = dispatch::event_sequence(&mut db, &changed);
    let history = dispatch::deadline_history(&mut db);
    let (store, old_pid) = timeout::open(&mut db);
    let name = application_name(&mut db, old_pid);
    let first = store.dispatch(request(1)).unwrap();
    assert_eq!(first.event.unwrap().sequence, sequence);
    assert_eq!((first.selected, first.inserted), (1, 1));
    assert!(!first.completed_scan);
    assert_eq!(first.progress.event.active_sequence, Some(sequence));
    assert_eq!(
        first.progress.event.after_deadline_id,
        Some(dispatch::id(20))
    );
    let committed_jobs = guards::jobs(&mut db);
    let audits = advanced_audits(&mut db);
    terminate(&mut db, old_pid);

    let next = eventually(store.as_ref(), request(100)).unwrap();
    assert_eq!(next.event.unwrap().sequence, sequence);
    assert_eq!((next.selected, next.inserted), (2, 2));
    assert!(next.completed_scan);
    assert_eq!(next.progress.event.completed_sequence, Some(sequence));
    assert_eq!(next.progress.event.active_sequence, None);
    assert_eq!(next.progress.event.after_deadline_id, None);
    let new_pid = replacement_pid(&mut db, &name, old_pid);
    let jobs = guards::jobs(&mut db);
    for prior in committed_jobs.as_array().unwrap() {
        assert!(jobs.as_array().unwrap().contains(prior));
    }
    assert_eq!(
        dispatch::event_jobs(&mut db, sequence),
        vec![dispatch::id(20), dispatch::id(40), dispatch::id(60)]
    );
    assert_eq!(advanced_audits(&mut db), audits + 1);
    assert_eq!(dispatch::deadline_history(&mut db), history);
    let committed = guards::snapshot(&mut db);
    for _ in 0..2 {
        let idle = store.dispatch(request(100)).unwrap();
        assert!(idle.event.is_none());
        assert_eq!((idle.selected, idle.inserted), (0, 0));
        assert_eq!(idle.progress, next.progress);
    }
    assert_eq!(guards::snapshot(&mut db), committed);
    timeout::same_connection(&mut db, new_pid);
}

#[derive(Clone, Copy, Debug)]
enum Corruption {
    WorkerGuard,
    MissingCursor,
}

fn alter(db: &mut dl::Fixture, corruption: Corruption, restore: Option<&Value>) {
    let mut tx = db.admin.transaction().unwrap();
    match corruption {
        Corruption::WorkerGuard => {
            let setting = if restore.is_some() {
                "ENABLE"
            } else {
                "DISABLE"
            };
            tx.batch_execute(&format!(
                "ALTER TABLE deadline_reevaluation_results {setting} TRIGGER deadline_worker_result_immutable"
            )).unwrap();
        }
        Corruption::MissingCursor => {
            tx.batch_execute(
                "ALTER TABLE deadline_dispatch_cursor DISABLE TRIGGER deadline_dispatch_protected",
            )
            .unwrap();
            if let Some(original) = restore {
                assert_eq!(tx.execute(
                    "INSERT INTO deadline_dispatch_cursor SELECT (jsonb_populate_record(NULL::deadline_dispatch_cursor,$1)).*",
                    &[original],
                ).unwrap(), 1);
            } else {
                assert_eq!(
                    tx.execute("DELETE FROM deadline_dispatch_cursor", &[])
                        .unwrap(),
                    1
                );
            }
            tx.batch_execute(
                "ALTER TABLE deadline_dispatch_cursor ENABLE TRIGGER deadline_dispatch_protected",
            )
            .unwrap();
        }
    }
    tx.commit().unwrap();
}

#[test]
fn reconnect_rejects_altered_worker_catalog_then_same_store_recovers_after_repair() {
    let _scenario = RECONNECT_SCENARIO
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    rejects_corruption_then_recovers(Corruption::WorkerGuard);
}

#[test]
fn reconnect_rejects_missing_cursor_inventory_then_same_store_recovers_after_restoration() {
    let _scenario = RECONNECT_SCENARIO
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    rejects_corruption_then_recovers(Corruption::MissingCursor);
}

fn rejects_corruption_then_recovers(corruption: Corruption) {
    let Some((mut db, changed, _)) = guards::setup(&[20, 40]) else {
        return;
    };
    let sequence = dispatch::event_sequence(&mut db, &changed);
    let (store, old_pid) = timeout::open(&mut db);
    let name = application_name(&mut db, old_pid);
    let before = guards::snapshot(&mut db);
    let cursor: Value = db
        .admin
        .query_one("SELECT to_jsonb(c) FROM deadline_dispatch_cursor c", &[])
        .unwrap()
        .get(0);
    terminate(&mut db, old_pid);
    alter(&mut db, corruption, None);
    if matches!(corruption, Corruption::MissingCursor) {
        let guarded: bool = db
            .admin
            .query_one(
                "SELECT bool_and(tgenabled IN ('O','A')) FROM pg_trigger
             WHERE tgrelid='deadline_dispatch_cursor'::regclass",
                &[],
            )
            .unwrap()
            .get(0);
        assert!(
            guarded,
            "inventory rejection must retain the real catalog guards"
        );
    }
    let altered = guards::snapshot(&mut db);
    let rejected = eventually(store.as_ref(), request(100));
    let rejected_again = eventually(store.as_ref(), request(100));
    let after_rejection = guards::snapshot(&mut db);
    // Restore the exact checkpoint and guards before asserting any outcome.
    alter(&mut db, corruption, Some(&cursor));
    assert_eq!(after_rejection, altered, "{corruption:?}");
    assert_eq!(guards::snapshot(&mut db), before, "{corruption:?}");
    for outcome in [rejected, rejected_again] {
        assert!(
            matches!(&outcome, Err(ApplicationError::InvalidConfiguration(_))),
            "reconnect accepted or bypassed {corruption:?}: {outcome:?}"
        );
    }
    let recovered = eventually(store.as_ref(), request(100)).unwrap();
    assert_eq!(recovered.event.unwrap().sequence, sequence);
    assert_eq!((recovered.selected, recovered.inserted), (2, 2));
    assert!(recovered.completed_scan);
    replacement_pid(&mut db, &name, old_pid);
    assert_eq!(
        dispatch::event_jobs(&mut db, sequence),
        vec![dispatch::id(20), dispatch::id(40)]
    );
    let committed = guards::snapshot(&mut db);
    assert!(store.dispatch(request(100)).unwrap().event.is_none());
    assert_eq!(guards::snapshot(&mut db), committed);
}

#[test]
fn reconnected_dispatcher_restores_lock_and_statement_budgets_before_dispatching_again() {
    let _scenario = RECONNECT_SCENARIO
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let Some((mut db, changed, _)) = guards::setup(&[20, 40]) else {
        return;
    };
    let sequence = dispatch::event_sequence(&mut db, &changed);
    let (store, old_pid) = timeout::open(&mut db);
    let name = application_name(&mut db, old_pid);
    terminate(&mut db, old_pid);
    let bootstrap = eventually(
        store.as_ref(),
        DeadlineDispatchRequest {
            stream: DeadlineDispatchStream::LegacyBootstrap,
            limit: DeadlineDispatchLimit::new(100).unwrap(),
        },
    )
    .unwrap();
    assert_eq!((bootstrap.selected, bootstrap.inserted), (2, 2));
    assert!(bootstrap.completed_scan);
    let pid = replacement_pid(&mut db, &name, old_pid);
    let before = guards::snapshot(&mut db);

    let mut held = db.admin.transaction().unwrap();
    held.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])
        .unwrap();
    let (receiver, worker) = timeout::start(Arc::clone(&store));
    let (bounded, outcome) = timeout::bounded_result(&mut db.control, pid, &receiver);
    held.rollback().unwrap();
    worker.join().unwrap();
    assert!(bounded, "the reconnected dispatcher lost its lock timeout");
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

    timeout::slow_audit(&mut db, sequence);
    let (receiver, worker) = timeout::start(Arc::clone(&store));
    let (bounded, outcome) = timeout::bounded_result(&mut db.control, pid, &receiver);
    worker.join().unwrap();
    let reached = timeout::slow_audit_reached(&mut db);
    let after_failure = guards::snapshot(&mut db);
    timeout::remove_slow_audit(&mut db);
    assert!(
        bounded,
        "the reconnected dispatcher lost its statement timeout"
    );
    assert!(
        reached,
        "the delayed audit must follow the job and cursor writes"
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
    let recovered = store.dispatch(request(100)).unwrap();
    assert_eq!(recovered.event.unwrap().sequence, sequence);
    assert_eq!((recovered.selected, recovered.inserted), (2, 2));
    assert!(recovered.completed_scan);
    assert_eq!(
        dispatch::event_jobs(&mut db, sequence),
        vec![dispatch::id(20), dispatch::id(40)]
    );
    timeout::same_connection(&mut db, pid);
}
