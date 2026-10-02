use super::password_reset_runtime_support::*;
use application::identity::{
    LoginChallengeIdentity, Principal, SessionIdentity, SessionPolicy, SessionStore,
};
use domain::crypto::DocumentHasher;
use domain::identity::{Role, UserId};
use infrastructure::{RedisSessionStore, RingSha256Hasher};
use redis::Commands;
use std::time::Duration;

#[test]
fn construction_rejects_zero_overflow_and_unbounded_windows_or_timeouts() {
    for (max, seconds) in [(0, 60), (1, 0), (1, 86401), (1, u64::MAX)] {
        assert!(PasswordResetRateLimit::new(max, seconds).is_err());
    }
    assert!(PasswordResetRateLimit::new(u32::MAX, 86400).is_ok());
    for (connect, io) in [
        (Duration::ZERO, Duration::from_secs(1)),
        (Duration::from_secs(1), Duration::ZERO),
    ] {
        assert!(RedisPasswordResetLimiter::connect(
            "redis://127.0.0.1/",
            policy(2, 1),
            connect,
            io
        )
        .is_err());
    }
    let error = RedisPasswordResetLimiter::connect(
        "not-a-url-private-marker",
        policy(2, 1),
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .err()
    .expect("invalid URL must fail");
    assert!(!error.to_string().contains("private-marker"));
}

fn corrupt(db: &mut Fixture, key: &str, condition: &str) {
    match condition {
        "no_ttl" => {
            db.connection.persist::<_, ()>(key).unwrap();
        }
        "type" => {
            db.connection.del::<_, ()>(key).unwrap();
            db.connection
                .lpush::<_, _, ()>(key, "private-corrupt-state")
                .unwrap();
        }
        "extra" => {
            db.connection
                .hset::<_, _, _, ()>(key, "extra", "1")
                .unwrap();
        }
        "missing" => {
            db.connection.hdel::<_, _, ()>(key, "version").unwrap();
        }
        "version" => {
            db.connection
                .hset::<_, _, _, ()>(key, "version", "2")
                .unwrap();
        }
        "limit" => {
            db.connection
                .hset::<_, _, _, ()>(key, "limit", "2")
                .unwrap();
        }
        "deadline" => {
            let deadline = db.deadline(key);
            db.expire_at(key, deadline - 1);
        }
        "future" => {
            let window: i64 = db.fields(key)["window_ms"].parse().unwrap();
            let opened = db.now() + 10000;
            db.connection
                .hset_multiple::<_, _, _, ()>(
                    key,
                    &[
                        ("opened_at_unix_ms", opened.to_string()),
                        ("expires_at_unix_ms", (opened + window).to_string()),
                    ],
                )
                .unwrap();
            db.expire_at(key, opened + window);
        }
        "policy" => {
            db.connection
                .hset::<_, _, _, ()>(key, "window_ms", "1000")
                .unwrap();
        }
        value => {
            db.connection
                .hset::<_, _, _, ()>(key, "count", value)
                .unwrap();
        }
    }
}

#[test]
fn corrupt_either_budget_fails_before_mutating_even_when_the_other_is_full() {
    for completion in [false, true] {
        for global in [false, true] {
            for condition in [
                "0",
                "01",
                "-1",
                "2",
                "9007199254740992",
                "no_ttl",
                "type",
                "extra",
                "missing",
                "version",
                "limit",
                "deadline",
                "future",
                "policy",
            ] {
                let mut db = Fixture::new();
                let (email, email_key) = db.email();
                let (digest, digest_key) = db.completion(31);
                let store = db.store(policy(1, 1));
                let admit = || {
                    if completion {
                        store.admit_completion(digest)
                    } else {
                        store.admit_request(&email)
                    }
                };
                assert!(admit().unwrap());
                let key = match (completion, global) {
                    (false, false) => email_key.as_str(),
                    (true, false) => digest_key.as_str(),
                    (false, true) => REQUEST_GLOBAL,
                    (true, true) => COMPLETION_GLOBAL,
                };
                corrupt(&mut db, key, condition);
                let before = db.snapshot();
                let result = admit();
                assert!(
                    result.is_err(),
                    "corrupt {condition} budget was treated as quota"
                );
                assert!(
                    db.snapshot() == before,
                    "corrupt state was changed or repaired"
                );
                let error = result.err().unwrap().to_string();
                assert!(!error.contains(&email) && !error.contains("private-corrupt-state"));
                if condition == "type" {
                    let other = match (completion, global) {
                        (false, false) => REQUEST_GLOBAL,
                        (true, false) => COMPLETION_GLOBAL,
                        (false, true) => email_key.as_str(),
                        (true, true) => digest_key.as_str(),
                    };
                    db.connection.del::<_, ()>(other).unwrap();
                    let before = db.snapshot();
                    assert!(admit().is_err());
                    assert!(
                        db.snapshot() == before,
                        "corrupt state created the absent other budget"
                    );
                }
            }
        }
    }
}

#[test]
fn changing_a_live_policy_cannot_reset_its_existing_budget() {
    let mut db = Fixture::new();
    let (email, _) = db.email();
    assert!(db.store(policy(2, 1)).admit_request(&email).unwrap());
    let before = db.snapshot();
    for changed in [
        PasswordResetRatePolicy::new(limit(3, 60), limit(1, 30), limit(2, 90), limit(1, 45)),
        PasswordResetRatePolicy::new(limit(2, 61), limit(1, 30), limit(2, 90), limit(1, 45)),
    ] {
        assert!(db.store(changed).admit_request(&email).is_err());
        assert!(
            db.snapshot() == before,
            "new configuration reset a live counter"
        );
    }
}

fn identity_key(kind: &str, bytes: &[u8]) -> String {
    format!(
        "identity:{kind}:{}",
        RingSha256Hasher.hash_bytes(bytes).to_hex()
    )
}

#[test]
fn reset_limits_preserve_existing_sessions_challenges_login_failures_and_totp_claims() {
    let mut db = Fixture::new();
    let (email, email_key) = db.email();
    let user = UserId::new();
    let sessions = RedisSessionStore::connect(&db.url).unwrap();
    let challenge = sessions
        .create_challenge(
            &LoginChallengeIdentity {
                user_id: user,
                auth_generation: 0,
            },
            60,
        )
        .unwrap();
    let challenge_key = db.track(identity_key("challenge", challenge.as_bytes()));
    let session = sessions
        .create_session(
            &SessionIdentity {
                principal: Principal {
                    id: user,
                    email: email.clone(),
                    role: Role::Litigator,
                },
                auth_generation: 0,
            },
            SessionPolicy::new(60, None).unwrap(),
        )
        .unwrap();
    let session_key = db.track(identity_key("session", session.access_token.as_bytes()));
    let failures_key = db.track(identity_key("password-failures", email.as_bytes()));
    sessions.record_password_failure(&email, 60).unwrap();
    let code = "123456";
    let mut claim = user.as_uuid().as_bytes().to_vec();
    claim.extend_from_slice(code.as_bytes());
    let totp_key = db.track(identity_key("totp-used", &claim));
    assert!(sessions.claim_totp(user, code, 60).unwrap());
    let keys = [challenge_key, session_key, failures_key, totp_key];
    let before: Vec<_> = keys.iter().map(|key| db.read(key)).collect();
    let store = db.store(policy(1, 1));
    assert!(store.admit_request(&email).unwrap());
    assert!(!store.admit_request(&email).unwrap());
    assert!(!email_key.contains(&email));
    assert!(db
        .fields(&email_key)
        .values()
        .all(|value| !value.contains(&email)));
    for (key, original) in keys.iter().zip(before) {
        assert!(
            db.read(key) == original,
            "reset altered other identity state or expiration"
        );
    }
    assert_eq!(sessions.failed_password_attempts(&email, 60).unwrap(), 1);
}
