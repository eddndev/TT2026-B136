use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use redis::Commands;

use super::{material, support::*};

#[test]
fn public_capture_roundtrips_exactly_with_independent_nonces_tokens_and_signed_deadline() {
    let mut db = Fixture::new();
    let store = db.store(policy(20, 10));
    let mut nonces = BTreeSet::new();
    let mut tokens = BTreeSet::new();
    for _ in 0..16 {
        let nonce = store.nonce().unwrap();
        assert!(
            nonces.insert(*nonce.as_bytes()),
            "nonce repeated within one sample"
        );
        let issued = db.now() / 1000 - 20;
        let mut value = material::capture(issued, 90);
        value.statement = material::statement(&value.context, nonce, issued, 90);
        let (token, key) = db.issue(&value, 90);
        assert!(
            tokens.insert(token.clone()),
            "opaque token repeated within one sample"
        );
        assert_ne!(
            URL_SAFE_NO_PAD.decode(&token).unwrap().as_slice(),
            nonce.as_bytes().as_slice(),
            "capture token must be independent of the public signing nonce"
        );
        assert_eq!(db.deadline(&key), (issued + 90) * 1000);
        let raw: String = db.connection.get(&key).unwrap();
        let stored: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(stored, material::wire(&value));
        assert_eq!(store.take(&token).unwrap(), Some(value));
        assert!(!db.connection.exists::<_, bool>(&key).unwrap());
        assert!(store.take(&token).unwrap().is_none());
    }
    assert!(!db.connection.exists::<_, bool>(START_GLOBAL).unwrap());
    assert!(!db.connection.exists::<_, bool>(PROOF_GLOBAL).unwrap());
}

#[test]
fn simultaneous_claims_across_instances_return_the_capture_to_exactly_one_caller() {
    let mut db = Fixture::new();
    let value = material::capture(db.now() / 1000, 90);
    let (token, key) = db.issue(&value, 90);
    let barrier = Arc::new(Barrier::new(9));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let barrier = barrier.clone();
            let token = token.clone();
            let store = db.store(policy(20, 10));
            std::thread::spawn(move || {
                barrier.wait();
                store.take(&token).unwrap()
            })
        })
        .collect();
    barrier.wait();
    let found: Vec<_> = workers
        .into_iter()
        .filter_map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(found, vec![value]);
    assert!(!db.connection.exists::<_, bool>(&key).unwrap());
}

#[test]
fn real_exclusive_expiry_removes_capture_without_reopening_its_proof_budget() {
    let mut db = Fixture::new();
    let store = db.store(policy(4, 1));
    let value = material::capture(db.now() / 1000, 2);
    let (token, key) = db.issue(&value, 2);
    let subject = db.reserve(proof_key(&token));
    store.admit_proof(&token).unwrap();
    let global = db.read(PROOF_GLOBAL);
    let before = db.read(&subject);
    let deadline = value.statement.expires_at_unix_seconds() * 1000;
    let until = Instant::now() + Duration::from_secs(3);
    while db.now() < deadline && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(db.now() >= deadline, "fixture did not reach signed expiry");
    assert!(store.take(&token).unwrap().is_none());
    exhausted(store.admit_proof(&token));
    assert_eq!(db.read(PROOF_GLOBAL), global);
    assert_eq!(db.read(&subject), before);
    assert!(!db.connection.exists::<_, bool>(&key).unwrap());
}

#[test]
fn missing_or_modified_absolute_expiration_is_consumed_without_returning_a_proof() {
    let mut db = Fixture::new();
    let store = db.store(policy(20, 10));
    let value = material::capture(db.now() / 1000, 90);
    for adjustment in [None, Some(-1), Some(1)] {
        let (token, key) = db.issue(&value, 90);
        if let Some(delta) = adjustment {
            db.expire(
                &key,
                value.statement.expires_at_unix_seconds() * 1000 + delta,
            );
        } else {
            db.connection.persist::<_, ()>(&key).unwrap();
        }
        assert!(store.take(&token).unwrap().is_none());
        assert!(
            !db.connection.exists::<_, bool>(&key).unwrap(),
            "invalid expiry remained replayable"
        );
        assert!(store.take(&token).unwrap().is_none());
    }
}

#[test]
fn unknown_well_shaped_token_is_absent_but_does_not_bypass_its_budget() {
    let mut db = Fixture::new();
    let (token, key) = db.token();
    let store = db.store(policy(2, 1));
    store.admit_proof(&token).unwrap();
    assert!(store.take(&token).unwrap().is_none());
    exhausted(store.admit_proof(&token));
    assert!(!db.connection.exists::<_, bool>(&key).unwrap());
}
