use super::*;
use postgres::{Client, NoTls};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[test]
fn an_event_waiting_for_the_audit_lock_cannot_commit_behind_a_confirmed_cursor() {
    let Some(mut db) = Fixture::new() else { return };
    record(&mut db, "calendar");
    record(&mut db, "resolution");
    let first = event(&mut db, "calendar");
    let second = event(&mut db, "resolution");
    db.admin.batch_execute("BEGIN; ALTER TABLE deadline_source_events DISABLE TRIGGER USER;
        DELETE FROM deadline_source_events; ALTER TABLE deadline_source_events ENABLE TRIGGER USER; COMMIT;
        BEGIN; SELECT pg_advisory_xact_lock(280603412820)").unwrap();
    let (pid_tx, pid_rx) = mpsc::channel();
    let (sequence_tx, sequence_rx) = mpsc::channel();
    let (commit_tx, commit_rx) = mpsc::channel();
    let url = db.runtime_url.clone();
    let worker = std::thread::spawn(move || {
        let mut client = Client::connect(&url, NoTls).unwrap();
        client
            .batch_execute("BEGIN; SET LOCAL statement_timeout='10s'")
            .unwrap();
        let pid: i32 = client
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        pid_tx.send(pid).unwrap();
        let sequence: i64 = client.query_one("INSERT INTO deadline_source_events(source_kind,source_id,revision,case_id,hearing_id,operation_id) VALUES($1,$2,$3,$4,$5,$6) RETURNING sequence",
            &[&second.kind,&second.id,&second.revision,&second.case,&second.hearing,&second.operation]).unwrap().get(0);
        sequence_tx.send(sequence).unwrap();
        commit_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        client.batch_execute("COMMIT").unwrap();
        sequence
    });
    let pid = pid_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let waited = waiting_for_audit_lock(&mut db.control, pid);
    replay_revision(&mut db, &first);
    let first_sequence: i64 = db
        .admin
        .query_one(
            "SELECT sequence FROM deadline_source_events WHERE source_kind='calendar'",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin.batch_execute("COMMIT").unwrap();
    let second_sequence = sequence_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let cursor: i64 = db
        .admin
        .query_one("SELECT max(sequence) FROM deadline_source_events", &[])
        .unwrap()
        .get(0);
    commit_tx.send(()).unwrap();
    assert_eq!(worker.join().unwrap(), second_sequence);
    assert!(
        waited,
        "second insert did not wait for the shared audit lock"
    );
    assert_eq!(cursor, first_sequence);
    assert!(second_sequence > cursor);
    let remaining = db
        .admin
        .query(
            "SELECT sequence FROM deadline_source_events WHERE sequence>$1 ORDER BY sequence",
            &[&cursor],
        )
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].get::<_, i64>(0), second_sequence);
    assert_events_match_sources(&mut db);
}

fn waiting_for_audit_lock(client: &mut Client, pid: i32) -> bool {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let waiting: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND NOT granted)", &[&pid]).unwrap().get(0);
        if waiting {
            return true;
        }
        if Instant::now() >= end {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
