mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_revalidation_support;

use application::{hearings::*, identity::Principal};
use domain::{
    clock::{Clock, OffsetDateTime},
    identity::Role,
};
use hearing_database_support::*;
use infrastructure::{PostgresHearingStore, RingSha256Hasher};
use std::sync::{
    atomic::{AtomicI64, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::{Duration, Instant};

struct AdjustableClock(AtomicI64);
impl Clock for AdjustableClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst)).unwrap()
    }
}

#[test]
fn committed_capture_time_is_observed_after_waiting_for_the_shared_audit_lock() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let command = schedule();
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", db.case, command.clone())
        .unwrap();
    let early = db.at.unix_timestamp();
    let later = early + 600;
    let clock = Arc::new(AdjustableClock(AtomicI64::new(early)));
    let store = Arc::new(
        PostgresHearingStore::open(&db.runtime_url, Arc::new(RingSha256Hasher), clock.clone())
            .unwrap(),
    );
    let (ready_tx, ready_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let resume_rx = Mutex::new(resume_rx);
    let interleaved = hearing_revalidation_support::interleaved(store, move || {
        ready_tx.send(()).unwrap();
        resume_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    });
    let workflow = HearingService::new(
        interleaved,
        Arc::new(case_stage_database_support::TestIdentity(Principal {
            id: db.owner,
            email: "session@example.test".into(),
            role: Role::Owner,
        })),
        case_stage_database_support::processor(),
        Arc::new(case_stage_database_support::FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        clock.clone(),
    );
    let case = db.case;
    let worker = std::thread::spawn(move || {
        workflow.submit("session", case, command, draft.submission_digest)
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    db.admin
        .batch_execute("SELECT pg_advisory_lock(280603412820)")
        .unwrap();
    resume_tx.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let observed = loop {
        let waiting: bool = db.admin.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')", &[&db.role]).unwrap().get(0);
        if waiting {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    clock.0.store(later, Ordering::SeqCst);
    db.admin
        .batch_execute("SELECT pg_advisory_unlock(280603412820)")
        .unwrap();
    assert!(
        observed,
        "prepared hearing did not wait for the shared audit lock"
    );
    let captured = worker.join().unwrap().unwrap();
    assert_eq!(captured.snapshot.recorded_at.unix_timestamp(), later);
    assert_eq!(captured.snapshot.recorded_at.nanosecond(), 0);
    assert_eq!(
        captured
            .snapshot
            .scheduling_context
            .administration_revision
            .get(),
        1
    );
}
