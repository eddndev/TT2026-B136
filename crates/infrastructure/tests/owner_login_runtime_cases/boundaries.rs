use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;

use super::{material, support::*};

#[test]
fn explicit_policy_and_timeout_bounds_reject_zero_overflow_and_invalid_transport() {
    for (maximum, seconds) in [(0, 60), (1, 0), (1, 86_401), (1, u64::MAX)] {
        assert!(OwnerLoginRateLimit::new(maximum, seconds).is_err());
    }
    assert!(OwnerLoginRateLimit::new(u32::MAX, 86_400).is_ok());
    for (connect, io) in [
        (Duration::ZERO, Duration::from_secs(1)),
        (Duration::from_secs(1), Duration::ZERO),
        (Duration::MAX, Duration::from_secs(1)),
        (Duration::from_secs(1), Duration::MAX),
    ] {
        assert!(
            RedisOwnerLoginRuntime::connect("redis://127.0.0.1/", policy(2, 1), connect, io)
                .is_err()
        );
    }
    let error = RedisOwnerLoginRuntime::connect(
        "private-invalid-url",
        policy(2, 1),
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .err()
    .expect("invalid URL must fail");
    assert!(!error.to_string().contains("private-invalid-url"));
}

#[test]
fn malformed_tokens_are_rejected_before_network_without_normalization() {
    let store = connect("redis://127.0.0.1:1/", policy(2, 1));
    let valid = URL_SAFE_NO_PAD.encode([255_u8; 32]);
    let mut noncanonical = valid.clone();
    noncanonical.pop();
    noncanonical.push('9');
    for token in [
        String::new(),
        Uuid::new_v4().to_string(),
        "a".repeat(42),
        "a".repeat(44),
        format!("{valid}="),
        format!(" {valid}"),
        valid.replace('_', "/"),
        noncanonical,
    ] {
        assert!(matches!(
            store.admit_proof(&token),
            Err(ApplicationError::InvalidCredentials)
        ));
        assert!(matches!(
            store.take(&token),
            Err(ApplicationError::InvalidCredentials)
        ));
    }
}

#[test]
fn creation_rejects_ttl_mismatch_future_expired_and_unrepresentable_windows_without_writes() {
    let mut db = Fixture::new();
    let store = db.store(policy(20, 10));
    let now = db.now() / 1000;
    let value = material::capture(now, 90);
    let before = db.keys();
    for ttl in [0, 89, 91, 301, u64::MAX] {
        assert!(store.create(&value, ttl).is_err());
        assert_eq!(db.keys(), before);
    }
    for issued in [now + 30, now - 90] {
        let changed = material::capture(issued, 90);
        assert!(store.create(&changed, 90).is_err());
        assert_eq!(db.keys(), before);
    }
    let mut overflow = value.clone();
    overflow.statement = material::statement(
        &overflow.context,
        *value.statement.nonce(),
        9_007_199_254_741,
        90,
    );
    assert!(store.create(&overflow, 90).is_err());
    assert_eq!(db.keys(), before);
    for duration in [1, 300] {
        if duration == 1 {
            let until = std::time::Instant::now() + Duration::from_secs(2);
            while db.now() % 1000 > 250 && std::time::Instant::now() < until {
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(
                db.now() % 1000 <= 250,
                "fixture could not enter a fresh second"
            );
        }
        let accepted = material::capture(db.now() / 1000, duration);
        let (token, _) = db.issue(&accepted, duration as u64);
        assert_eq!(store.take(&token).unwrap(), Some(accepted));
    }
}

#[test]
fn oversized_or_inconsistent_public_material_is_rejected_before_set() {
    let mut db = Fixture::new();
    let store = db.store(policy(20, 10));
    let original = material::capture(db.now() / 1000, 90);
    let before = db.keys();
    for component in ["leaf", "root", "crl"] {
        for length in [
            0,
            if component == "crl" {
                1_048_577
            } else {
                16_385
            },
        ] {
            let mut value = original.clone();
            match component {
                "leaf" => {
                    value.context.certificate.der = vec![7; length];
                    value.context.certificate.fingerprint =
                        RingSha256Hasher.hash_bytes(&value.context.certificate.der);
                }
                "root" => {
                    value.context.trust.inspection.root_der = vec![8; length];
                    value.context.trust.inspection.root_fingerprint =
                        RingSha256Hasher.hash_bytes(&value.context.trust.inspection.root_der);
                }
                _ => {
                    value.context.trust.inspection.crl_der = vec![9; length];
                    value.context.trust.inspection.crl_digest =
                        RingSha256Hasher.hash_bytes(&value.context.trust.inspection.crl_der);
                }
            }
            value.statement = material::statement(
                &value.context,
                *original.statement.nonce(),
                original.statement.issued_at_unix_seconds(),
                90,
            );
            assert!(store.create(&value, 90).is_err());
            assert_eq!(db.keys(), before);
        }
    }
    let mut other = original.clone();
    other.context.account.auth_generation -= 1;
    assert!(store.create(&other, 90).is_err());
    other = original.clone();
    other.context.trust.inspection.crl_der.push(1);
    assert!(store.create(&other, 90).is_err());
    other = original;
    other.context.certificate.summary.subject = "x".repeat(4 * 1024 * 1024 + 1);
    assert!(store.create(&other, 90).is_err());
    assert_eq!(db.keys(), before);
}
