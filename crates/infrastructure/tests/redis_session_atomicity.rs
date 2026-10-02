use std::env;

use application::identity::{
    LoginChallengeIdentity, Principal, SessionIdentity, SessionPolicy, SessionStore,
};
use application::ApplicationError;
use domain::crypto::DocumentHasher;
use domain::identity::{Role, UserId};
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

fn session_identity() -> SessionIdentity {
    SessionIdentity {
        principal: Principal {
            id: UserId::new(),
            email: "owner@example.com".to_string(),
            role: Role::Owner,
        },
        auth_generation: 0,
    }
}

fn session_key(token: &str) -> String {
    format!(
        "identity:session:{}",
        RingSha256Hasher::new()
            .hash_bytes(token.as_bytes())
            .to_hex()
    )
}

#[test]
fn legacy_sessions_without_deadlines_require_reauthentication() {
    let Some(url) = redis_url() else { return };
    let store = RedisSessionStore::connect(&url).unwrap();
    let identity = session_identity();
    let policy = SessionPolicy::new(60, None).unwrap();
    let token = store
        .create_session(&identity, policy)
        .unwrap()
        .access_token;
    let key = session_key(&token);
    let legacy = serde_json::json!({
        "principal": {
            "id": identity.principal.id,
            "email": identity.principal.email,
            "role": "owner"
        },
        "auth_generation": 0
    })
    .to_string();
    let mut connection = redis::Client::open(url).unwrap().get_connection().unwrap();
    connection.set_ex::<_, _, ()>(&key, legacy, 60).unwrap();
    let initial_ttl: i64 = connection.pttl(&key).unwrap();

    let result = store.find_session(&token, policy);
    let remaining_ttl: redis::RedisResult<i64> = connection.pttl(&key);
    connection.del::<_, ()>(&key).unwrap();

    assert!(
        result.unwrap().is_none(),
        "legacy identity was authenticated"
    );
    let remaining_ttl = remaining_ttl.unwrap();
    assert!(
        remaining_ttl == -2 || (0..=initial_ttl).contains(&remaining_ttl),
        "legacy session expiration was extended"
    );
}

#[test]
fn sessions_without_expiration_are_rejected_without_repairing_their_ttl() {
    let Some(url) = redis_url() else { return };
    let store = RedisSessionStore::connect(&url).unwrap();
    let policy = SessionPolicy::new(60, None).unwrap();
    let token = store
        .create_session(&session_identity(), policy)
        .unwrap()
        .access_token;
    let key = session_key(&token);
    let mut connection = redis::Client::open(url).unwrap().get_connection().unwrap();
    let removed_expiration: bool = connection.persist(&key).unwrap();

    let result = store.find_session(&token, policy);
    let remaining_ttl: redis::RedisResult<i64> = connection.pttl(&key);
    connection.del::<_, ()>(&key).unwrap();

    assert!(
        removed_expiration,
        "fixture session did not have an expiration"
    );
    assert!(
        result.unwrap().is_none(),
        "session without TTL was authenticated"
    );
    assert!(
        matches!(remaining_ttl.unwrap(), -2 | -1),
        "session lookup assigned a new expiration"
    );
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
    let identity = LoginChallengeIdentity {
        user_id: user,
        auth_generation: 0,
    };
    let token = store.create_challenge(&identity, 30).unwrap();
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
    assert!(results == vec![identity]);
    assert!(store.take_challenge(&token).unwrap().is_none());
}
