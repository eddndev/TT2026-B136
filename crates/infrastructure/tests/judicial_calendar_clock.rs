mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod judicial_calendar_database_support;
mod judicial_calendar_interleaved_support;
use application::{identity::Principal, judicial_calendars::*};
use domain::{
    clock::{Clock, OffsetDateTime},
    identity::Role,
};
use infrastructure::{PostgresJudicialCalendarStore, RingSha256Hasher};
use judicial_calendar_database_support::*;
use judicial_calendar_interleaved_support::*;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::Duration;
struct AdjustableClock(AtomicI64);
impl Clock for AdjustableClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst)).unwrap()
    }
}
#[test]
fn calendar_capture_uses_clock_after_waiting_for_audit_lock() {
    let Some(mut db) = Fixture::new() else { return };
    let command = publish();
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", command.clone())
        .unwrap();
    let clock = Arc::new(AdjustableClock(AtomicI64::new(db.at.unix_timestamp())));
    let store = Arc::new(
        PostgresJudicialCalendarStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            clock.clone(),
        )
        .unwrap(),
    );
    let (ready_tx, ready_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let resume_rx = Mutex::new(resume_rx);
    let adapter = interleaved(store, move || {
        ready_tx.send(()).unwrap();
        resume_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
    });
    let workflow = JudicialCalendarService::new(
        adapter,
        Arc::new(case_stage_database_support::TestIdentity(Principal {
            id: db.owner,
            email: "session@example.test".into(),
            role: Role::Owner,
        })),
        Arc::new(RingSha256Hasher),
        clock.clone(),
    );
    let worker =
        std::thread::spawn(move || workflow.submit("session", command, draft.submission_digest));
    ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    db.admin
        .batch_execute("SELECT pg_advisory_lock(280603412820)")
        .unwrap();
    resume_tx.send(()).unwrap();
    let waiting = wait_for_lock(&mut db);
    let later = db.at.unix_timestamp() + 600;
    clock.0.store(later, Ordering::SeqCst);
    db.admin
        .batch_execute("SELECT pg_advisory_unlock(280603412820)")
        .unwrap();
    assert!(waiting, "calendar commit did not wait for the audit lock");
    let result = worker.join().unwrap().unwrap();
    assert_eq!(result.recorded_at.unix_timestamp(), later);
    assert_eq!(result.recorded_at.nanosecond(), 0);
}
