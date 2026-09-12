use std::env;

use application::identity::SessionStore;
use application::ApplicationError;
use domain::crypto::DocumentHasher;
use domain::identity::UserId;
use infrastructure::{RedisSessionStore, RingSha256Hasher};
use redis::Commands;

fn redis_url() -> Option<String> {
    let url = env::var("IDENTITY_TEST_REDIS_URL").ok();
    if url.is_none() {
        eprintln!("skipping Redis atomicity test: IDENTITY_TEST_REDIS_URL is unset");
    }
    url
}

fn failure_key(email: &str) -> String {
    format!(
        "identity:password-failures:{}",
        RingSha256Hasher::new()
            .hash_bytes(email.as_bytes())
            .to_hex()
    )
}

#[test]
fn a_failure_counter_without_expiration_is_repaired() {
    let Some(url) = redis_url() else { return };
    let store = RedisSessionStore::connect(&url).unwrap();
    let email = format!("{}@example.com", uuid::Uuid::new_v4());
    let key = failure_key(&email);
    let mut connection = redis::Client::open(url).unwrap().get_connection().unwrap();
    connection.set::<_, _, ()>(&key, 1).unwrap();

    assert_eq!(store.record_password_failure(&email, 30).unwrap(), 2);
    let ttl: i64 = connection.ttl(&key).unwrap();
    assert!((1..=30).contains(&ttl), "counter expiration was {ttl}");
}

#[test]
fn subsequent_failures_do_not_extend_the_original_window() {
    let Some(url) = redis_url() else { return };
    let store = RedisSessionStore::connect(&url).unwrap();
    let email = format!("{}@example.com", uuid::Uuid::new_v4());
    let key = failure_key(&email);
    let mut connection = redis::Client::open(url).unwrap().get_connection().unwrap();
    assert_eq!(store.record_password_failure(&email, 30).unwrap(), 1);
    connection.expire::<_, ()>(&key, 5).unwrap();

    assert_eq!(store.record_password_failure(&email, 30).unwrap(), 2);
    let ttl: i64 = connection.ttl(&key).unwrap();
    assert!(
        (1..=5).contains(&ttl),
        "failure extended the window to {ttl}"
    );
}

#[test]
fn reading_a_locked_legacy_counter_repairs_its_missing_expiration() {
    let Some(url) = redis_url() else { return };
    let store = RedisSessionStore::connect(&url).unwrap();
    let email = format!("{}@example.com", uuid::Uuid::new_v4());
    let key = failure_key(&email);
    let mut connection = redis::Client::open(url).unwrap().get_connection().unwrap();
    connection.set::<_, _, ()>(&key, 5).unwrap();

    assert_eq!(store.failed_password_attempts(&email, 30).unwrap(), 5);
    let ttl: i64 = connection.ttl(&key).unwrap();
    assert!(
        (1..=30).contains(&ttl),
        "locked counter expiration was {ttl}"
    );
}

#[test]
fn reading_failure_counts_preserves_a_live_window_and_keeps_absent_keys_absent() {
    let Some(url) = redis_url() else { return };
    let store = RedisSessionStore::connect(&url).unwrap();
    let email = format!("{}@example.com", uuid::Uuid::new_v4());
    let key = failure_key(&email);
    let mut connection = redis::Client::open(url).unwrap().get_connection().unwrap();

    assert_eq!(store.failed_password_attempts(&email, 30).unwrap(), 0);
    assert!(!connection.exists::<_, bool>(&key).unwrap());
    connection.set_ex::<_, _, ()>(&key, 5, 5).unwrap();
    for _ in 0..2 {
        assert_eq!(store.failed_password_attempts(&email, 30).unwrap(), 5);
        let ttl: i64 = connection.ttl(&key).unwrap();
        assert!((1..=5).contains(&ttl), "read extended the window to {ttl}");
    }
}

#[test]
fn invalid_expiration_is_rejected_before_mutating_the_counter() {
    let Some(url) = redis_url() else { return };
    let store = RedisSessionStore::connect(&url).unwrap();
    let email = format!("{}@example.com", uuid::Uuid::new_v4());
    for ttl in [0, i64::MAX as u64, u64::MAX] {
        assert!(matches!(
            store.record_password_failure(&email, ttl),
            Err(ApplicationError::InvalidInput(_))
        ));
        assert_eq!(store.failed_password_attempts(&email, 30).unwrap(), 0);
    }
}

#[test]
fn concurrent_callers_can_take_a_challenge_exactly_once() {
    let Some(url) = redis_url() else { return };
    let store = RedisSessionStore::connect(&url).unwrap();
    let user = UserId::new();
    let token = store.create_challenge(user, 30).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let store = RedisSessionStore::connect(&url).unwrap();
            let token = token.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.take_challenge(&token).unwrap()
            })
        })
        .collect();
    let results: Vec<_> = workers
        .into_iter()
        .filter_map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(results, vec![user]);
    assert_eq!(store.take_challenge(&token).unwrap(), None);
}
