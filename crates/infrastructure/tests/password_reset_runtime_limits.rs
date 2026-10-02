use super::password_reset_runtime_support::*;
use redis::Commands;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

#[test]
fn request_rejection_neither_charges_the_other_budget_nor_extends_its_window() {
    let mut db = Fixture::new();
    let (first, first_key) = db.email();
    let (second, second_key) = db.email();
    let (third, _) = db.email();
    let store = db.store(policy(2, 1));
    assert!(store.admit_request(&first).unwrap());
    let before = db.snapshot();
    assert!(!store.admit_request(&first).unwrap());
    assert!(
        db.snapshot() == before,
        "per-email rejection changed Redis state or expiry"
    );
    assert!(store.admit_request(&second).unwrap());
    let before = db.snapshot();
    assert!(!store.admit_request(&third).unwrap());
    assert!(
        db.snapshot() == before,
        "global rejection charged a new email"
    );
    drop(store);
    assert!(!db.store(policy(2, 1)).admit_request(&third).unwrap());
    assert!(
        db.snapshot() == before,
        "opening another client reset the global budget"
    );
    for (key, count, window, max) in [
        (REQUEST_GLOBAL, "2", "60000", "2"),
        (first_key.as_str(), "1", "30000", "1"),
        (second_key.as_str(), "1", "30000", "1"),
    ] {
        let fields = db.fields(key);
        assert_eq!(fields.len(), 6);
        assert_eq!(fields["version"], "1");
        assert_eq!(fields["limit"], max);
        assert_eq!(fields["window_ms"], window);
        assert_eq!(fields["count"], count);
        let opened = fields["opened_at_unix_ms"].parse::<i64>().unwrap();
        let deadline = fields["expires_at_unix_ms"].parse::<i64>().unwrap();
        assert_eq!(deadline - opened, window.parse::<i64>().unwrap());
        assert_eq!(db.deadline(key), deadline);
        assert!(db.now() < deadline);
    }
    assert_eq!(db.deadline(COMPLETION_GLOBAL), -2);
    db.connection.del::<_, ()>(REQUEST_GLOBAL).unwrap();
    let before = db.snapshot();
    assert!(!db.store(policy(2, 1)).admit_request(&first).unwrap());
    assert!(
        db.snapshot() == before,
        "full email budget recreated an absent global counter"
    );
}

#[test]
fn completion_budgets_are_independent_and_rejection_does_not_charge_a_digest() {
    let mut db = Fixture::new();
    let (email, _) = db.email();
    let (first, first_key) = db.completion(11);
    let (second, _) = db.completion(12);
    let (third, _) = db.completion(13);
    let store = db.store(policy(2, 1));
    assert!(store.admit_request(&email).unwrap());
    let request_before = db.read(REQUEST_GLOBAL);
    assert!(store.admit_completion(first).unwrap());
    let before = db.snapshot();
    assert!(!store.admit_completion(first).unwrap());
    assert!(
        db.snapshot() == before,
        "per-digest rejection changed a budget"
    );
    assert!(store.admit_completion(second).unwrap());
    let before = db.snapshot();
    assert!(!store.admit_completion(third).unwrap());
    assert!(
        db.snapshot() == before,
        "global completion rejection created a digest counter"
    );
    assert!(
        db.read(REQUEST_GLOBAL) == request_before,
        "completion charged request budget"
    );
    assert_eq!(db.fields(COMPLETION_GLOBAL)["window_ms"], "90000");
    assert_eq!(db.fields(&first_key)["window_ms"], "45000");
    assert_ne!(first_key, email_key(&first.to_hex()));
}

fn burst(db: &Fixture, emails: Vec<String>) -> usize {
    let barrier = Arc::new(Barrier::new(emails.len() + 1));
    let workers: Vec<_> = emails
        .into_iter()
        .map(|email| {
            let barrier = barrier.clone();
            let store = db.store(policy(7, 3));
            std::thread::spawn(move || {
                barrier.wait();
                store.admit_request(&email).unwrap()
            })
        })
        .collect();
    barrier.wait();
    workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .filter(|admitted| *admitted)
        .count()
}

#[test]
fn independent_clients_share_atomic_per_email_and_global_capacity() {
    let mut db = Fixture::new();
    let (email, key) = db.email();
    assert_eq!(burst(&db, vec![email; 16]), 3);
    assert_eq!(db.fields(&key)["count"], "3");
    assert_eq!(db.fields(REQUEST_GLOBAL)["count"], "3");
    let others = (0..8).map(|_| db.email().0).collect();
    assert_eq!(burst(&db, others), 4);
    assert_eq!(db.fields(REQUEST_GLOBAL)["count"], "7");
    assert_eq!(db.fields(&key)["count"], "3");
}

#[test]
fn real_subject_expiry_opens_a_new_window_without_restarting_the_global_window() {
    let mut db = Fixture::new();
    let (email, key) = db.email();
    let store = db.store(policy(4, 1));
    assert!(store.admit_request(&email).unwrap());
    let original_global_deadline = db.deadline(REQUEST_GLOBAL);
    let before = db.snapshot();
    assert!(!store.admit_request(&email).unwrap());
    assert!(
        db.snapshot() == before,
        "denied request extended the existing window"
    );
    let deadline = db.now() + 150;
    db.connection
        .hset_multiple::<_, _, _, ()>(
            &key,
            &[
                ("opened_at_unix_ms", (deadline - 30000).to_string()),
                ("expires_at_unix_ms", deadline.to_string()),
            ],
        )
        .unwrap();
    db.expire_at(&key, deadline);
    let until = Instant::now() + Duration::from_secs(2);
    while db.connection.exists::<_, bool>(&key).unwrap() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !db.connection.exists::<_, bool>(&key).unwrap(),
        "fixture window did not expire"
    );
    assert!(store.admit_request(&email).unwrap());
    assert_eq!(db.fields(&key)["count"], "1");
    assert_eq!(db.fields(REQUEST_GLOBAL)["count"], "2");
    assert_eq!(db.deadline(REQUEST_GLOBAL), original_global_deadline);
    assert!(db.deadline(&key) > deadline);
}
