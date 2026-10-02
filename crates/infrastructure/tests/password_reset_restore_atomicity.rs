use super::password_reset_backend_support::*;
use super::password_reset_restore_support::*;
use postgres::{Client, NoTls};
use std::time::{Duration, Instant};

#[test]
fn audit_insert_timeout_rolls_back_cancellation_and_retry_commits_once() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let before = snapshot(&mut db);
    let application = format!("reset_restore_{}", uuid::Uuid::new_v4().simple());
    let url = operation_url(&db, &application, None);
    let retry = repeat_request(&request);
    let mut blocker = Client::connect(&db.admin_url, NoTls).unwrap();
    let mut held = blocker.transaction().unwrap();
    // SHARE admits validation reads but blocks the later audit INSERT.
    held.batch_execute("LOCK TABLE audit_events IN SHARE MODE")
        .unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = sender.send(invalidate_restored_password_resets(&url, request));
    });
    let deadline = Instant::now() + Duration::from_secs(8);
    let reached_audit = loop {
        let waiting: bool = db
            .admin
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1
             AND wait_event_type='Lock' AND wait_event='relation'
             AND query ILIKE '%INSERT%audit_events%')",
                &[&application],
            )
            .unwrap()
            .get(0);
        if waiting || Instant::now() >= deadline {
            break waiting;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let failed = receiver.recv_timeout(Duration::from_secs(8));
    held.rollback().unwrap();
    worker.join().unwrap();
    assert!(
        reached_audit,
        "operation did not reach the blocked audit INSERT"
    );
    assert!(
        failed.unwrap().is_err(),
        "audit timeout unexpectedly committed"
    );
    assert_private_unchanged(&mut db, &before);
    let result =
        invalidate_restored_password_resets(&db.admin_url, repeat_request(&retry)).unwrap();
    assert!(result.applied);
    assert_eq!(result.invalidated, 1);
    assert_restore_receipt(&mut db, retry.operation_id, &result);
}

struct EmptySchema {
    connection: Client,
    name: String,
}

impl Drop for EmptySchema {
    fn drop(&mut self) {
        let _ = self
            .connection
            .batch_execute(&format!("DROP SCHEMA {} CASCADE", self.name));
    }
}

#[test]
fn restoration_locks_and_validates_the_actual_schema_after_an_empty_search_path_prefix() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    let mut prefix = EmptySchema {
        connection: Client::connect(&db.admin_url, NoTls).unwrap(),
        name: format!("reset_restore_empty_{}", uuid::Uuid::new_v4().simple()),
    };
    prefix
        .connection
        .batch_execute(&format!("CREATE SCHEMA {}", prefix.name))
        .unwrap();
    let application = format!("reset_restore_prefix_{}", uuid::Uuid::new_v4().simple());
    let url = operation_url(&db, &application, Some(&prefix.name));
    let mut connection = Client::connect(&url, NoTls).unwrap();
    let row = connection
        .query_one(
            "SELECT current_schema(),n.nspname::text
        FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
        WHERE c.oid='password_reset_capabilities'::regclass",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), prefix.name);
    assert_eq!(row.get::<_, String>(1), db.schema);
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let operation = request.operation_id;
    let before = snapshot(&mut db);
    let mut blocker = Client::connect(&db.admin_url, NoTls).unwrap();
    let pid: i32 = blocker
        .query_one("SELECT pg_backend_pid()", &[])
        .unwrap()
        .get(0);
    let mut held = blocker.transaction().unwrap();
    // Match the schema migration exclusion defined in docs/adr/0048-schema-scoped-migration-lock.md.
    held.query_one(
        "SELECT pg_advisory_xact_lock($1::integer,n.oid::integer)
        FROM pg_namespace n WHERE n.nspname=$2",
        &[&0x43415345_i32, &db.schema],
    )
    .unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = sender.send(invalidate_restored_password_resets(&url, request));
    });
    let deadline = Instant::now() + Duration::from_secs(8);
    let reached_schema = loop {
        let waiting: bool = db
            .admin
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1
             AND wait_event_type='Lock' AND wait_event='advisory'
             AND $2=ANY(pg_blocking_pids(pid)))",
                &[&application, &pid],
            )
            .unwrap()
            .get(0);
        if waiting || Instant::now() >= deadline {
            break waiting;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let while_locked = snapshot(&mut db);
    held.rollback().unwrap();
    let result = receiver.recv_timeout(Duration::from_secs(8));
    worker.join().unwrap();
    assert!(
        reached_schema,
        "operation did not wait for the actual relation schema lock"
    );
    assert!(
        while_locked == before,
        "operation mutated rows before schema exclusion"
    );
    let result = result.unwrap().unwrap();
    assert!(result.applied);
    assert_eq!(result.invalidated, 1);
    assert_restore_receipt(&mut db, operation, &result);
    let objects: i64 = prefix
        .connection
        .query_one(
            "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
         WHERE n.nspname=$1",
            &[&prefix.name],
        )
        .unwrap()
        .get(0);
    assert_eq!(
        objects, 0,
        "invalidation unexpectedly applied DDL in the search prefix"
    );
    assert!(repository.inspect(digest(1)).unwrap().is_none());
}
