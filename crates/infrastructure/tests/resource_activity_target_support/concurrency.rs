use super::*;
use domain::clock::{Clock, OffsetDateTime};
use infrastructure::{PostgresResourceActivityStore, RingSha256Hasher};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

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
                    .recv_timeout(Duration::from_secs(10))
                    .is_err())
        {
            self.expired.store(true, Ordering::SeqCst);
        }
        self.at
    }
}
fn named_url(base: &str, prefix: &str) -> (String, String) {
    let name = format!("{prefix}_{}", uuid::Uuid::new_v4().simple());
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
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        let waiting: bool = db.control.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock' AND wait_event='advisory' AND $2=ANY(pg_blocking_pids(pid)))", &[&writer, &reader]).unwrap().get(0);
        if waiting || Instant::now() >= until {
            return waiting;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn unlink_waits_for_inverse_read_and_next_observation_sees_its_exact_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let linked = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        captures.resource.id,
        captures.link(),
    );
    db.admin.batch_execute(&format!("ALTER ROLE {} SET statement_timeout='8s'; ALTER ROLE {} SET idle_in_transaction_session_timeout='12s'", db.role, db.role)).unwrap();
    let (reader_url, reader_name) = named_url(&db.runtime_url, "activity_target_reader");
    let (writer_url, writer_name) = named_url(&db.runtime_url, "activity_target_writer");
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
        PostgresResourceActivityStore::open(&reader_url, Arc::new(RingSha256Hasher), clock.clone())
            .unwrap();
    let writer_store = Arc::new(
        PostgresResourceActivityStore::open(
            &writer_url,
            Arc::new(RingSha256Hasher),
            Arc::new(case_stage_database_support::FixedClock(db.at)),
        )
        .unwrap(),
    );
    let writer = service_with_store(&db, db.owner, Role::Owner, writer_store);
    let reader_pid = pid(&mut db, &reader_name);
    let writer_pid = pid(&mut db, &writer_name);
    let (case, owner, at) = (db.case, db.owner, db.at);
    let wanted = target(&captures);
    let command = unlink(&linked, captures.head.revision);
    let (waited, first, removed) = thread::scope(|scope| {
        let read =
            scope.spawn(|| reader.list_for_target(owner, case, wanted, query(10, None, None), at));
        ready_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("inverse read must acquire audited lock");
        let unlink = scope.spawn(|| persist(&writer, case, captures.resource.id, command));
        let waited = waits_on_reader(&mut db, reader_pid, writer_pid);
        let _ = resume_tx.send(());
        (waited, read.join().unwrap(), unlink.join().unwrap())
    });
    assert!(
        waited,
        "unlink must wait for the inverse read audit transaction"
    );
    assert!(!clock.expired.load(Ordering::SeqCst));
    assert_eq!(first.unwrap().associations[0].association, linked);
    let next = reader
        .list_for_target(owner, case, wanted, query(10, None, None), at)
        .unwrap();
    assert_eq!(next.associations[0].association, removed);
    assert_eq!(removed.status, ResourceActivityStatus::Unlinked);
    assert!(reader
        .list_for_target(
            owner,
            case,
            wanted,
            query(10, None, Some(ResourceActivityStatus::Linked)),
            at
        )
        .unwrap()
        .associations
        .is_empty());
}
