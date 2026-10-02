use std::sync::{Arc, Barrier};

use application::identity::SessionStore;
use infrastructure::RedisSessionStore;

use super::redis_session_support::{number, Fixture};

#[test]
fn concurrent_activity_keeps_the_latest_deadline_and_original_identity() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    fixture.seed(10_000);
    let before = fixture.raw();
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let store = RedisSessionStore::connect(&fixture.url).unwrap();
            let token = fixture.token.clone();
            let identity = fixture.identity.clone();
            let policy = fixture.policy;
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store
                    .record_activity(&token, &identity, policy)
                    .unwrap()
                    .unwrap()
            })
        })
        .collect();
    let states: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    let after = fixture.raw();
    assert_eq!(after["identity_json"], before["identity_json"]);
    assert_eq!(
        after["absolute_expires_at_unix_ms"],
        before["absolute_expires_at_unix_ms"]
    );
    assert_eq!(
        number(&after, "idle_expires_at_unix_ms"),
        states
            .iter()
            .map(|state| state.idle_expires_at_unix_ms.unwrap())
            .max()
            .unwrap()
    );
    assert!(number(&after, "idle_expires_at_unix_ms") > number(&before, "idle_expires_at_unix_ms"));
}

#[test]
fn revocation_wins_after_concurrent_activity_and_late_activity_cannot_resurrect() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    fixture.seed(10_000);
    let barrier = Arc::new(Barrier::new(2));
    let store = RedisSessionStore::connect(&fixture.url).unwrap();
    let token = fixture.token.clone();
    let identity = fixture.identity.clone();
    let policy = fixture.policy;
    let activity_barrier = Arc::clone(&barrier);
    let activity = std::thread::spawn(move || {
        activity_barrier.wait();
        store.record_activity(&token, &identity, policy).unwrap()
    });
    barrier.wait();
    fixture.store.revoke_session(&fixture.token).unwrap();
    activity.join().unwrap();
    assert!(!fixture.exists());
    assert!(fixture
        .store
        .record_activity(&fixture.token, &fixture.identity, fixture.policy)
        .unwrap()
        .is_none());
    assert!(!fixture.exists());
}
