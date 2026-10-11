use application::identity::invitations::InvitationLimiter;
use domain::crypto::Sha256Digest;
use infrastructure::identity::RedisInvitationLimiter;
use std::{sync::Arc, time::Duration};

#[test]
fn shared_invitation_attempt_budget_is_atomic_and_separate_from_other_flows() {
    let url = std::env::var("INVITATION_TEST_REDIS_URL")
        .or_else(|_| std::env::var("IDENTITY_TEST_REDIS_URL"))
        .expect("an isolated invitation Redis is required");
    let make = || {
        RedisInvitationLimiter::connect(
            &url,
            20,
            5,
            300,
            Duration::from_secs(1),
            Duration::from_secs(1),
        )
        .unwrap()
    };
    let first = Arc::new(make());
    let digest = Sha256Digest::from_array([41; 32]);
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let limiter = first.clone();
            std::thread::spawn(move || limiter.admit(digest).unwrap())
        })
        .collect();
    let admitted = workers
        .into_iter()
        .map(|w| usize::from(w.join().unwrap()))
        .sum::<usize>();
    assert_eq!(admitted, 5);
    assert!(!make().admit(digest).unwrap());
    assert!(make().admit(Sha256Digest::from_array([42; 32])).unwrap());
    let mut connection = redis::Client::open(url).unwrap().get_connection().unwrap();
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("identity:invitation:v1:*")
        .query(&mut connection)
        .unwrap();
    assert_eq!(keys.len(), 3);
    for key in &keys {
        let ttl: i64 = redis::cmd("TTL").arg(key).query(&mut connection).unwrap();
        assert!((1..=300).contains(&ttl));
    }
    let other: Vec<String> = redis::cmd("KEYS")
        .arg("identity:password-reset:*")
        .query(&mut connection)
        .unwrap();
    assert!(other.is_empty());
    redis::cmd("DEL")
        .arg(keys)
        .query::<()>(&mut connection)
        .unwrap();
}

#[test]
fn limiter_rejects_invalid_settings_and_does_not_expose_connection_errors() {
    for (global, token, window) in [(0, 1, 1), (1, 0, 1), (1, 1, 0), (1, 1, 86401)] {
        assert!(RedisInvitationLimiter::connect(
            "redis://127.0.0.1/",
            global,
            token,
            window,
            Duration::from_secs(1),
            Duration::from_secs(1)
        )
        .is_err());
    }
    let limiter = RedisInvitationLimiter::connect(
        "redis://private-credential@127.0.0.1:1/",
        1,
        1,
        60,
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .unwrap();
    let error = limiter
        .admit(Sha256Digest::from_array([1; 32]))
        .unwrap_err()
        .to_string();
    assert!(!error.contains("private-credential"));
}
