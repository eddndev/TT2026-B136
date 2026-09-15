mod case_administration_support;

use case_administration_support::Fixture;
use domain::audit::AuditLog;
use infrastructure::PostgresAuditLog;
use std::time::{Duration, Instant};

#[test]
fn audited_transaction_sees_a_committed_predecessor_after_waiting_with_repeatable_read_default() {
    let Some(mut f) = Fixture::new() else { return };
    f.admin
        .batch_execute(&format!(
            "ALTER ROLE {} SET default_transaction_isolation='repeatable read'",
            f.role
        ))
        .unwrap();
    let mut worker = PostgresAuditLog::open(&f.runtime_url).unwrap();
    let mut predecessor = PostgresAuditLog::open(&f.runtime_url).unwrap();
    f.admin
        .batch_execute("SELECT pg_advisory_lock(280603412820)")
        .unwrap();
    let at = f.at;
    let handle =
        std::thread::spawn(move || worker.append("owner@example.test", "second", "cases", at));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let waiting: bool = f.admin.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')", &[&f.role]).unwrap().get(0);
        if waiting {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "audit writer did not wait on shared mutation lock"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // The holding session can reacquire its own advisory lock and commit a predecessor.
    let timestamp = "2025-01-01T00:00:00.123456789Z";
    f.admin.execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES(0,$1,'owner@example.test','first','cases',$2)", &[&timestamp, &&[1u8;32][..]]).unwrap();
    f.admin
        .batch_execute("SELECT pg_advisory_unlock(280603412820)")
        .unwrap();
    let second = handle.join().unwrap().unwrap();
    assert_eq!(second.event.sequence, 1);
    let third = predecessor
        .append("owner@example.test", "third", "cases", at)
        .unwrap();
    assert_eq!(third.event.sequence, 2);
}
