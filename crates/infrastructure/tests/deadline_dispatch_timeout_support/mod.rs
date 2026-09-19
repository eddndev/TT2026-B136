use crate::{case_stage_database_support::FixedClock, deadline_backend_support as dl};
use application::{deadline_dispatch::*, ApplicationError};
use infrastructure::{PostgresDeadlineDispatchStore, RingSha256Hasher};
use postgres::Client;
use std::{
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
    time::Duration,
};
use uuid::Uuid;

pub type Outcome = Result<DeadlineDispatchBatch, ApplicationError>;

pub fn open(db: &mut dl::Fixture) -> (Arc<PostgresDeadlineDispatchStore>, i32) {
    let application = format!("deadline_dispatch_timeout_{}", Uuid::new_v4().simple());
    let mut url = reqwest::Url::parse(&db.runtime_url).unwrap();
    url.query_pairs_mut()
        .append_pair("application_name", &application);
    let store = Arc::new(
        PostgresDeadlineDispatchStore::open(
            url.as_str(),
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    );
    let pid = db
        .admin
        .query_one(
            "SELECT pid FROM pg_stat_activity WHERE usename=$1 AND application_name=$2",
            &[&db.role, &application],
        )
        .unwrap()
        .get(0);
    (store, pid)
}

pub fn request() -> DeadlineDispatchRequest {
    DeadlineDispatchRequest {
        stream: DeadlineDispatchStream::Events,
        limit: DeadlineDispatchLimit::new(100).unwrap(),
    }
}

pub fn start(
    store: Arc<PostgresDeadlineDispatchStore>,
) -> (mpsc::Receiver<Outcome>, JoinHandle<()>) {
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        let result = store.dispatch(request());
        sender.send(result).unwrap();
    });
    (receiver, worker)
}

/// Cancellation is a test watchdog, never a successful implementation result.
pub fn bounded_result(
    control: &mut Client,
    pid: i32,
    receiver: &mpsc::Receiver<Outcome>,
) -> (bool, Outcome) {
    match receiver.recv_timeout(Duration::from_secs(15)) {
        Ok(outcome) => (true, outcome),
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("dispatch worker disconnected"),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            control
                .query_one("SELECT pg_cancel_backend($1)", &[&pid])
                .unwrap();
            if let Ok(outcome) = receiver.recv_timeout(Duration::from_secs(10)) {
                return (false, outcome);
            }
            control
                .query_one("SELECT pg_terminate_backend($1)", &[&pid])
                .unwrap();
            let outcome = receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("cancelled test backend must finish in finite time");
            (false, outcome)
        }
    }
}

pub fn same_connection(db: &mut dl::Fixture, pid: i32) {
    let present: bool = db
        .admin
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND usename=$2)",
            &[&pid, &db.role],
        )
        .unwrap()
        .get(0);
    assert!(
        present,
        "the same store connection must survive its database timeout"
    );
}

pub fn slow_audit(db: &mut dl::Fixture, sequence: u64) {
    db.admin.batch_execute(&format!(
        "CREATE SEQUENCE dispatch_timeout_reached;
        GRANT USAGE ON SEQUENCE dispatch_timeout_reached TO {};
        CREATE FUNCTION slow_deadline_dispatch_audit() RETURNS trigger
        LANGUAGE plpgsql SECURITY INVOKER AS $$ BEGIN
            IF NOT EXISTS(SELECT 1 FROM deadline_reevaluation_jobs WHERE event_sequence={sequence})
                OR NOT EXISTS(SELECT 1 FROM deadline_dispatch_cursor WHERE completed_event_sequence={sequence}) THEN
                RAISE EXCEPTION 'slow audit must follow job and cursor writes';
            END IF;
            PERFORM nextval('dispatch_timeout_reached');
            PERFORM pg_catalog.pg_sleep(30);
            RETURN NEW;
        END; $$;
        CREATE TRIGGER slow_deadline_dispatch_audit BEFORE INSERT ON audit_events
            FOR EACH ROW WHEN (NEW.action='deadline.dispatch_advanced')
            EXECUTE FUNCTION slow_deadline_dispatch_audit()", db.role
    )).unwrap();
}

pub fn slow_audit_reached(db: &mut dl::Fixture) -> bool {
    db.admin
        .query_one("SELECT is_called FROM dispatch_timeout_reached", &[])
        .unwrap()
        .get(0)
}

pub fn remove_slow_audit(db: &mut dl::Fixture) {
    db.admin
        .batch_execute(
            "DROP TRIGGER slow_deadline_dispatch_audit ON audit_events;
        DROP FUNCTION slow_deadline_dispatch_audit(); DROP SEQUENCE dispatch_timeout_reached",
        )
        .unwrap();
}
