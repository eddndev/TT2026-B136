use super::password_reset_backend_support::*;
use std::time::{Duration, Instant};

fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn expiry_is_rechecked_after_waiting_for_the_audit_mutation_lock() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    let issued = issue(&repository, target, 1);
    let observed = repository.inspect(digest(1)).unwrap().unwrap();
    db.admin.execute(
        "UPDATE password_reset_capabilities SET expires_at=clock_timestamp()+interval '1 second' WHERE id=$1",
        &[&issued.id.as_uuid()],
    ).unwrap();
    let before = snapshot(&mut db);
    let role = db.role.clone();
    let mut holder = db.control.transaction().unwrap();
    holder
        .query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])
        .unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        sender
            .send(repository.complete(complete(observed, 1, "too-late-hash")))
            .unwrap();
    });
    assert!(
        wait_until(|| {
            db.admin.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')",
        &[&role],
    ).unwrap().get(0)
        }),
        "consumer never reached the held mutation lock"
    );
    assert!(
        wait_until(|| db
            .admin
            .query_one(
                "SELECT clock_timestamp()>=expires_at FROM password_reset_capabilities WHERE id=$1",
                &[&issued.id.as_uuid()],
            )
            .unwrap()
            .get(0)),
        "stored server expiry was not reached"
    );
    holder.commit().unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap(),
        ResetCompletion::Rejected
    );
    worker.join().unwrap();
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn exhausted_user_counters_reject_without_consuming_the_capability() {
    let mut db = fixture();
    let target = account(&mut db, "client", true);
    let repository = store(&db);
    db.admin
        .batch_execute("ALTER TABLE users DISABLE TRIGGER USER")
        .unwrap();
    db.admin
        .execute(
            "UPDATE users SET revision=$2,auth_generation=$2 WHERE id=$1",
            &[&target.as_uuid(), &i64::MAX],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE users ENABLE TRIGGER USER")
        .unwrap();
    issue(&repository, target, 1);
    let observed = repository.inspect(digest(1)).unwrap().unwrap();
    let before = snapshot(&mut db);
    assert_eq!(
        repository
            .complete(complete(observed, 1, "overflow-hash"))
            .unwrap(),
        ResetCompletion::Rejected
    );
    assert_eq!(snapshot(&mut db), before);
}
