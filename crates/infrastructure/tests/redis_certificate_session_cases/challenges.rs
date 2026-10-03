use std::sync::{Arc, Barrier};

use redis::Commands;

use super::support::*;

#[test]
fn password_records_keep_their_exact_format_and_cannot_be_upgraded_by_certificate_activity() {
    let mut db = Fixture::new();
    let identity = db.challenge().identity;
    for generalized in [false, true] {
        let token = db.store.create_challenge(&identity, 30).unwrap();
        token_shape(&token);
        let key = db.track("challenge", &token);
        let raw: String = db.connection.get(&key).unwrap();
        assert_eq!(raw, serde_json::to_string(&identity).unwrap());
        if generalized {
            assert_eq!(
                db.store.take_mfa_challenge(&token).unwrap(),
                Some(MfaChallenge::Password(identity.clone()))
            );
        } else {
            assert_eq!(
                db.store.take_challenge(&token).unwrap(),
                Some(identity.clone())
            );
        }
        assert!(db.store.take_mfa_challenge(&token).unwrap().is_none());
    }
    let grant = db.store.create_session(&db.identity, db.policy).unwrap();
    let key = db.track("session", &grant.access_token);
    assert_eq!(grant.state.authentication, SessionAuthentication::Password);
    let fields = db.fields(&key);
    assert_eq!(fields.len(), 8);
    assert_eq!(fields["version"], "1");
    assert_eq!(
        fields["identity_json"],
        serde_json::to_string(&db.identity).unwrap()
    );
    let before = db.snapshot(&key);
    assert!(db
        .store
        .record_certificate_activity(&grant.access_token, &db.identity, &db.origin, db.policy)
        .unwrap()
        .is_none());
    assert!(
        db.snapshot(&key) == before,
        "certificate activity altered a password record"
    );
    assert_eq!(
        db.store
            .find_session(&grant.access_token, db.policy)
            .unwrap()
            .unwrap()
            .authentication,
        SessionAuthentication::Password
    );
}

#[test]
fn certificate_mfa_has_an_exact_deadline_and_only_one_claimant_without_password_downgrade() {
    let mut db = Fixture::new();
    let challenge = db.challenge();
    let token = db.store.create_certificate_challenge(&challenge).unwrap();
    token_shape(&token);
    let key = db.track("challenge", &token);
    let raw: String = db.connection.get(&key).unwrap();
    let wire: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        wire,
        json!({ "version": 2, "identity": challenge.identity,
        "authentication": authentication(&challenge.provenance),
        "expires_at_unix_seconds": challenge.expires_at_unix_seconds })
    );
    assert_eq!(db.deadline(&key), challenge.expires_at_unix_seconds * 1000);
    assert!(
        !raw.contains(&token),
        "bearer token must not be stored in its value"
    );
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let store = connect(&db.url);
            let token = token.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store.take_mfa_challenge(&token).unwrap()
            })
        })
        .collect();
    let claimed: Vec<_> = workers
        .into_iter()
        .filter_map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        claimed,
        vec![MfaChallenge::Certificate(challenge.clone().into())]
    );
    assert!(!db.connection.exists::<_, bool>(&key).unwrap());
    assert!(db.store.take_mfa_challenge(&token).unwrap().is_none());
    let second = db.store.create_certificate_challenge(&challenge).unwrap();
    assert!(
        token != second,
        "new challenges must use separate opaque tokens"
    );
    let second_key = db.track("challenge", &second);
    assert!(db.store.take_challenge(&second).unwrap().is_none());
    assert!(!db.connection.exists::<_, bool>(&second_key).unwrap());
    assert!(db.store.take_mfa_challenge(&second).unwrap().is_none());
}

#[test]
fn malformed_origins_and_unproved_expirations_are_consumed_without_authentication() {
    let mut db = Fixture::new();
    let challenge = db.challenge();
    let token = db.store.create_certificate_challenge(&challenge).unwrap();
    let key = db.track("challenge", &token);
    let raw: String = db.connection.get(&key).unwrap();
    let good: Value = serde_json::from_str(&raw).unwrap();
    let mut invalid = vec![
        json!(null),
        json!([]),
        json!([
            challenge.identity.user_id,
            challenge.identity.auth_generation
        ]),
    ];
    for (pointer, value) in [
        ("/version", json!(1)),
        ("/version", json!(2.0)),
        ("/authentication", json!(null)),
        ("/authentication/kind", json!("password")),
        ("/authentication/kind", json!("unknown")),
        (
            "/authentication/provenance/crl_digest",
            json!("not-a-digest"),
        ),
        ("/authentication/provenance/trust_revision", json!(0)),
        (
            "/authentication/provenance/principal/role",
            json!("litigator"),
        ),
        (
            "/identity/auth_generation",
            json!(challenge.identity.auth_generation - 1),
        ),
        ("/expires_at_unix_seconds", json!(db.now() / 1000 - 1)),
    ] {
        let mut changed = good.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        invalid.push(changed);
    }
    for pointer in [
        "",
        "/identity",
        "/authentication",
        "/authentication/provenance",
        "/authentication/provenance/principal",
    ] {
        let mut changed = good.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unrecognized".into(), json!(true));
        invalid.push(changed);
    }
    let mut missing = good.clone();
    missing["authentication"]["provenance"]
        .as_object_mut()
        .unwrap()
        .remove("crl_digest");
    invalid.push(missing);
    let mut bare = serde_json::to_value(&challenge.identity).unwrap();
    bare["authentication"] = json!(null);
    invalid.push(bare);
    for value in invalid {
        for password_only in [false, true] {
            db.string(
                &key,
                &value.to_string(),
                challenge.expires_at_unix_seconds * 1000,
            );
            let rejected = if password_only {
                db.store.take_challenge(&token).unwrap().is_none()
            } else {
                db.store.take_mfa_challenge(&token).unwrap().is_none()
            };
            assert!(rejected, "malformed challenge was admitted");
            assert!(
                !db.connection.exists::<_, bool>(&key).unwrap(),
                "rejected challenge was replayable"
            );
        }
    }
    for expiration in [
        None,
        Some(challenge.expires_at_unix_seconds * 1000 + 1),
        Some(db.now() - 1),
    ] {
        db.string(&key, &raw, challenge.expires_at_unix_seconds * 1000);
        if let Some(deadline) = expiration {
            db.expire(&key, deadline);
        } else {
            db.connection.persist::<_, ()>(&key).unwrap();
        }
        assert!(db.store.take_mfa_challenge(&token).unwrap().is_none());
        assert!(!db.connection.exists::<_, bool>(&key).unwrap());
    }
}
