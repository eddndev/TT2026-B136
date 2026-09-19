mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod procedural_fact_backend_support;
#[allow(dead_code)]
mod procedural_fact_concurrency_support;

use application::{
    deadline_currentness::DeadlineFreshness::{Changed, Current},
    deadline_tracking::TrackingDependency,
    deadlines::*,
    procedural_facts::ProceduralFactWorkflow,
};
use case_stage_database_support::FixedClock;
use deadline_backend_support as dl;
use domain::{
    clock::{Clock, OffsetDateTime},
    identity::Role,
};
use infrastructure::{
    procedural_fact_postgres::PostgresProceduralFactStore, PostgresDeadlineStore, RingSha256Hasher,
};
use postgres::Client;
use procedural_fact_backend_support as facts;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

const READY_WAIT: Duration = Duration::from_secs(10);
const GATE_WAIT: Duration = Duration::from_secs(20);

struct PausedClock {
    at: OffsetDateTime,
    entered: AtomicBool,
    ready: mpsc::SyncSender<()>,
    resume: Mutex<mpsc::Receiver<()>>,
    expired: AtomicBool,
}

impl Clock for PausedClock {
    fn now(&self) -> OffsetDateTime {
        if !self.entered.swap(true, Ordering::SeqCst)
            && (self.ready.send(()).is_err()
                || self.resume.lock().unwrap().recv_timeout(GATE_WAIT).is_err())
        {
            self.expired.store(true, Ordering::SeqCst);
        }
        self.at
    }
}

struct Completed(mpsc::Sender<()>);
impl Drop for Completed {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

fn named_url(base: &str, prefix: &str) -> (String, String) {
    let name = format!("{prefix}_{}", Uuid::new_v4().simple());
    let mut url = reqwest::Url::parse(base).unwrap();
    url.query_pairs_mut()
        .append_pair("application_name", &name)
        .append_pair("connect_timeout", "5");
    (url.to_string(), name)
}

fn backend_pid(control: &mut Client, role: &str, name: &str) -> i32 {
    control
        .query_one(
            "SELECT pid FROM pg_stat_activity WHERE usename=$1 AND application_name=$2",
            &[&role, &name],
        )
        .unwrap()
        .get(0)
}

fn wait_for_writer_lock(
    control: &mut Client,
    writer_name: &str,
    writer_pid: i32,
    reader_pid: i32,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let observed: bool = control
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
             WHERE application_name=$1 AND pid=$2 AND wait_event_type='Lock'
               AND wait_event='advisory' AND $3=ANY(pg_blocking_pids(pid)))",
                &[&writer_name, &writer_pid, &reader_pid],
            )
            .map_err(|error| error.to_string())?
            .get(0);
        if observed {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("source writer did not wait on the current reader's audit lock".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

// Cancellation is cleanup on failure, never evidence that the required lock existed.
fn completed_without_cancellation(
    control: &mut Client,
    pid: i32,
    done: &mpsc::Receiver<()>,
) -> bool {
    if done.recv_timeout(Duration::from_secs(15)).is_ok() {
        return true;
    }
    let _ = control.query_one("SELECT pg_cancel_backend($1)", &[&pid]);
    if done.recv_timeout(Duration::from_secs(3)).is_err() {
        let _ = control.query_one("SELECT pg_terminate_backend($1)", &[&pid]);
        let _ = done.recv_timeout(Duration::from_secs(3));
    }
    false
}

#[test]
fn current_read_holds_verified_heads_until_its_audit_commits_before_a_source_writer() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let profile = dl::profile(&db);
    let source = dl::source(&db);
    let base = dl::persist(
        &dl::service(&db, db.owner, Role::Owner),
        db.case,
        dl::human(
            dl::command(&db, &profile, &source),
            Some(dl::FOLLOW_RESOLUTION),
        ),
    );
    let history = dl::snapshot(&mut db)["revisions"].clone();
    let initial_audit: i64 = db
        .admin
        .query_one("SELECT COALESCE(max(sequence),0) FROM audit_events", &[])
        .unwrap()
        .get(0);
    db.admin
        .batch_execute(&format!(
            "ALTER ROLE {} SET statement_timeout='10s';
         ALTER ROLE {} SET idle_in_transaction_session_timeout='20s'",
            db.role, db.role,
        ))
        .unwrap();
    db.control
        .batch_execute("SET statement_timeout='2s'")
        .unwrap();

    let (writer_url, writer_name) = named_url(&db.runtime_url, "deadline_source_writer");
    let writer_store = Arc::new(
        PostgresProceduralFactStore::open(
            &writer_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    );
    let writer_pid = backend_pid(&mut db.control, &db.role, &writer_name);
    let (writer_ready_tx, writer_ready_rx) = mpsc::sync_channel(1);
    let (write_tx, write_rx) = mpsc::sync_channel(1);
    let write_rx = Mutex::new(write_rx);
    let writer_expired = Arc::new(AtomicBool::new(false));
    let timed_out = writer_expired.clone();
    let writer_store =
        procedural_fact_concurrency_support::before_commit(writer_store, move |_| {
            if writer_ready_tx.send(()).is_err()
                || write_rx.lock().unwrap().recv_timeout(GATE_WAIT).is_err()
            {
                timed_out.store(true, Ordering::SeqCst);
            }
        });
    let writer = facts::service_with_store(&db, writer_store, db.owner, Role::Owner);
    let command = facts::correct(&source);
    let draft = writer.prepare("session", db.case, command.clone()).unwrap();

    let (reader_ready_tx, reader_ready_rx) = mpsc::sync_channel(1);
    let (read_tx, read_rx) = mpsc::sync_channel(1);
    let clock = Arc::new(PausedClock {
        at: db.at,
        entered: AtomicBool::new(false),
        ready: reader_ready_tx,
        resume: Mutex::new(read_rx),
        expired: AtomicBool::new(false),
    });
    let (reader_url, reader_name) = named_url(&db.runtime_url, "deadline_current_reader");
    let reader = Arc::new(
        PostgresDeadlineStore::open(&reader_url, Arc::new(RingSha256Hasher), clock.clone())
            .unwrap(),
    );
    let reader_pid = backend_pid(&mut db.control, &db.role, &reader_name);
    let (owner, case, id) = (db.owner, db.case, base.id);
    let read_store = reader.clone();

    let (coordination, finished, first_read, changed_source) = thread::scope(|scope| {
        let (write_done_tx, write_done_rx) = mpsc::channel();
        let writer_thread = scope.spawn(move || {
            let _completed = Completed(write_done_tx);
            writer.submit("session", case, command, draft.submission_digest)
        });
        let writer_ready = writer_ready_rx.recv_timeout(READY_WAIT);
        let (read_done_tx, read_done_rx) = mpsc::channel();
        let reader_thread = scope.spawn(move || {
            let _completed = Completed(read_done_tx);
            read_store.current(owner, case, id)
        });
        let coordination = (|| -> Result<(), String> {
            writer_ready.map_err(|error| format!("source preparation gate: {error}"))?;
            reader_ready_rx
                .recv_timeout(READY_WAIT)
                .map_err(|error| format!("current read clock gate: {error}"))?;
            write_tx.send(()).map_err(|error| error.to_string())?;
            wait_for_writer_lock(&mut db.control, &writer_name, writer_pid, reader_pid)
        })();
        // Release both gates on every coordination outcome before joining either thread.
        let _ = write_tx.try_send(());
        let _ = read_tx.try_send(());
        let read_finished =
            completed_without_cancellation(&mut db.control, reader_pid, &read_done_rx);
        let write_finished =
            completed_without_cancellation(&mut db.control, writer_pid, &write_done_rx);
        let first_read = reader_thread.join();
        let changed_source = writer_thread.join();
        (
            coordination,
            read_finished && write_finished,
            first_read,
            changed_source,
        )
    });

    // All assertions occur after both scoped workers have been released and joined.
    coordination.unwrap();
    assert!(
        finished,
        "a concurrent operation required watchdog cancellation"
    );
    assert!(!clock.expired.load(Ordering::SeqCst));
    assert!(!writer_expired.load(Ordering::SeqCst));
    let first_read = first_read.unwrap().unwrap();
    let changed_source = changed_source.unwrap().unwrap();
    assert_eq!(first_read.detail(), &base);
    assert_eq!(first_read.operational().freshness(), Current);
    assert_eq!(first_read.operational().checked_at(), Some(db.at));
    assert_eq!(
        first_read.operational().due_at(),
        base.calculation.result.due_at()
    );
    assert_eq!(
        changed_source.snapshot.metadata().revision,
        source.snapshot.metadata().revision.next().unwrap()
    );
    let actions: Vec<String> = db
        .admin
        .query(
            "SELECT action FROM audit_events WHERE sequence>$1 ORDER BY sequence",
            &[&initial_audit],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(
        actions,
        ["deadline.current_read", "procedural_fact.corrected"]
    );

    let next = reader.current(db.owner, db.case, base.id).unwrap();
    assert_eq!(next.detail(), &base);
    assert_eq!(next.operational().freshness(), Changed);
    assert_eq!(
        next.operational().changed_dependencies(),
        &[TrackingDependency::Source]
    );
    assert_eq!(next.operational().due_at(), None);
    assert_eq!(dl::snapshot(&mut db)["revisions"], history);
    assert_eq!(
        reader
            .get(db.owner, db.case, base.id, Some(base.revision), db.at)
            .unwrap(),
        base
    );
}
