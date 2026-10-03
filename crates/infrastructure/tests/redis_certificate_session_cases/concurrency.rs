use std::sync::{Arc, Barrier};

use redis::Commands;

use super::support::*;

#[test]
fn concurrent_certificate_activity_preserves_the_origin_and_greatest_admitted_deadline() {
    let mut db = Fixture::new();
    let ceiling = db.now() + 90_123;
    let (token, key, _) = db.issue(ceiling);
    db.age(&key, 10_000);
    let before = db.fields(&key);
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let store = connect(&db.url);
            let token = token.clone();
            let identity = db.identity.clone();
            let origin = db.origin.clone();
            let policy = db.policy;
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store
                    .record_certificate_activity(&token, &identity, &origin, policy)
                    .unwrap()
                    .unwrap()
            })
        })
        .collect();
    let states: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    let after = db.fields(&key);
    for field in [
        "identity_json",
        "authentication_json",
        "ceiling_unix_ms",
        "absolute_expires_at_unix_ms",
    ] {
        assert_eq!(after[field], before[field]);
    }
    assert_eq!(
        db.deadline(&key),
        states
            .iter()
            .map(|state| state.idle_expires_at_unix_ms.unwrap())
            .max()
            .unwrap()
    );
    assert_eq!(
        number(&after, "last_activity_unix_ms"),
        states
            .iter()
            .map(|state| state.server_now_unix_ms)
            .max()
            .unwrap()
    );
    for state in states {
        assert_eq!(state.identity, db.identity);
        assert_eq!(
            state.authentication,
            SessionAuthentication::Certificate(db.origin.clone().into())
        );
        assert!(state.idle_expires_at_unix_ms.unwrap() <= ceiling);
    }
}

#[test]
fn missing_expired_and_concurrently_revoked_certificate_bearers_never_resurrect() {
    let mut db = Fixture::new();
    let (absent, absent_key) = db.reserve("session");
    assert!(db
        .store
        .record_certificate_activity(&absent, &db.identity, &db.origin, db.policy)
        .unwrap()
        .is_none());
    assert!(!db.connection.exists::<_, bool>(&absent_key).unwrap());
    let ceiling = db.now() + 90_123;
    let (expired, expired_key, _) = db.issue(ceiling);
    let past = db.now() - 1;
    db.expire(&expired_key, past);
    assert!(db
        .store
        .find_session(&expired, db.policy)
        .unwrap()
        .is_none());
    assert!(db
        .store
        .record_certificate_activity(&expired, &db.identity, &db.origin, db.policy)
        .unwrap()
        .is_none());
    assert!(!db.connection.exists::<_, bool>(&expired_key).unwrap());
    let (token, key, _) = db.issue(ceiling);
    let barrier = Arc::new(Barrier::new(2));
    let activity_barrier = Arc::clone(&barrier);
    let store = connect(&db.url);
    let identity = db.identity.clone();
    let origin = db.origin.clone();
    let policy = db.policy;
    let worker_token = token.clone();
    let worker = std::thread::spawn(move || {
        activity_barrier.wait();
        store
            .record_certificate_activity(&worker_token, &identity, &origin, policy)
            .unwrap()
    });
    barrier.wait();
    db.store.revoke_session(&token).unwrap();
    worker.join().unwrap();
    assert!(!db.connection.exists::<_, bool>(&key).unwrap());
    assert!(db.store.find_session(&token, db.policy).unwrap().is_none());
    assert!(db
        .store
        .record_certificate_activity(&token, &db.identity, &db.origin, db.policy)
        .unwrap()
        .is_none());
    assert!(!db.connection.exists::<_, bool>(&key).unwrap());
}
