use super::*;
use domain::clock::Clock;
use infrastructure::PostgresCaseRepository;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex,
    },
    thread,
    time::{Duration as Wait, Instant},
};
use uuid::Uuid;

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
                || self
                    .resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Wait::from_secs(10))
                    .is_err())
        {
            self.expired.store(true, Ordering::SeqCst);
        }
        self.at
    }
}
fn named_url(base: &str, prefix: &str) -> (String, String) {
    let name = format!("{prefix}_{}", Uuid::new_v4().simple());
    let mut url = reqwest::Url::parse(base).unwrap();
    url.query_pairs_mut().append_pair("application_name", &name);
    (url.to_string(), name)
}
fn pid(db: &mut Fixture, name: &str) -> i32 {
    db.control
        .query_one(
            "SELECT pid FROM pg_stat_activity WHERE usename=$1 AND application_name=$2",
            &[&db.role, &name],
        )
        .unwrap()
        .get(0)
}
fn waits_on_reader(db: &mut Fixture, reader: i32, writer: i32) -> bool {
    let until = Instant::now() + Wait::from_secs(3);
    loop {
        let waiting: bool = db.control.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock' AND wait_event='advisory' AND $2=ANY(pg_blocking_pids(pid)))", &[&writer, &reader]).unwrap().get(0);
        if waiting || Instant::now() >= until {
            return waiting;
        }
        thread::sleep(Wait::from_millis(10));
    }
}
#[test]
fn revocation_waits_for_a_committed_snapshot_and_the_next_read_has_no_case_data() {
    let Some(mut db) = fixture() else { return };
    let member = db.user("litigator", true);
    db.admin.batch_execute(&format!("ALTER ROLE {} SET statement_timeout='8s'; ALTER ROLE {} SET idle_in_transaction_session_timeout='12s'", db.role, db.role)).unwrap();
    let (reader_url, reader_name) = named_url(&db.runtime_url, "dashboard_reader");
    let (writer_url, writer_name) = named_url(&db.runtime_url, "dashboard_revoker");
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (resume_tx, resume_rx) = mpsc::channel();
    let clock = Arc::new(PausedClock {
        at: db.at,
        entered: AtomicBool::new(false),
        ready: ready_tx,
        resume: Mutex::new(resume_rx),
        expired: AtomicBool::new(false),
    });
    let reader =
        PostgresDashboardStore::open(&reader_url, Arc::new(RingSha256Hasher), clock.clone())
            .unwrap();
    let writer = PostgresCaseRepository::open(&writer_url, Arc::new(RingSha256Hasher)).unwrap();
    let reader_pid = pid(&mut db, &reader_name);
    let writer_pid = pid(&mut db, &writer_name);
    let (case, owner, at) = (db.case, db.owner, db.at);
    let (waited, first, removed) = thread::scope(|scope| {
        let read = scope.spawn(|| reader.read(member));
        ready_rx
            .recv_timeout(Wait::from_secs(3))
            .expect("reader must acquire audited snapshot lock");
        let revoke = scope.spawn(|| writer.remove_member(case, member, owner, at));
        let waited = waits_on_reader(&mut db, reader_pid, writer_pid);
        let _ = resume_tx.send(());
        (waited, read.join().unwrap(), revoke.join().unwrap())
    });
    assert!(waited, "revocation must wait on the snapshot audit lock");
    assert!(!clock.expired.load(Ordering::SeqCst));
    assert_eq!(first.unwrap().active_cases, 1);
    removed.unwrap();
    let next = reader.read(member).unwrap();
    assert_eq!(next.active_cases, 0);
    assert_eq!(next.pending_contracts, 0);
    assert!(next.workload.is_empty());
    assert_eq!(audits(&mut db), 2);
}
