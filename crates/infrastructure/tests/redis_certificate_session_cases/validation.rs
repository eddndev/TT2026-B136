use redis::Commands;

use super::support::*;

#[test]
fn invalid_origins_identity_pairs_and_ceiling_inputs_create_no_redis_state() {
    let mut db = Fixture::new();
    let (_, sentinel) = db.reserve("certificate-fixture-sentinel");
    db.connection
        .set_ex::<_, _, ()>(&sentinel, "preserve unrelated state", 600)
        .unwrap();
    let sentinel_before = db.snapshot(&sentinel);
    let before = db.keys();
    let now = db.now();
    let mut invalid = Vec::new();
    for case in 0..10 {
        let mut origin = db.origin.clone();
        match case {
            0 => origin.principal.role = Role::Litigator,
            1 => origin.principal.id = UserId::from_uuid(uuid::Uuid::nil()),
            2 => origin.binding_id = uuid::Uuid::nil(),
            3 => origin.deployment_id = uuid::Uuid::nil(),
            4 => origin.trust_revision = 0,
            5 => origin.auth_generation = i64::MAX as u64 + 1,
            6 => origin.valid_from_unix_seconds = now / 1000 + 600,
            7 => origin.valid_until_unix_seconds = now / 1000 - 1,
            8 => origin.valid_until_unix_seconds = MAX_EXACT / 1000 + 1,
            _ => origin.valid_until_unix_seconds = i64::MAX,
        }
        invalid.push(origin);
    }
    for origin in invalid {
        let identity = SessionIdentity {
            principal: origin.principal.clone(),
            auth_generation: origin.auth_generation,
        };
        let value = CertificateMfaChallenge {
            identity: LoginChallengeIdentity {
                user_id: identity.principal.id,
                auth_generation: identity.auth_generation,
            },
            provenance: origin.clone(),
            expires_at_unix_seconds: now / 1000 + 60,
        };
        assert!(
            db.store.create_certificate_challenge(&value).is_err(),
            "invalid origin created MFA state"
        );
        assert!(
            db.store
                .create_certificate_session(&identity, &origin, db.policy, now + 60_000)
                .is_err(),
            "invalid origin created a session"
        );
        assert_eq!(db.keys(), before);
    }
    for ceiling in [
        now - 1,
        db.origin.valid_until_unix_seconds * 1000 + 1,
        MAX_EXACT + 1,
        i64::MAX,
    ] {
        assert!(db
            .store
            .create_certificate_session(&db.identity, &db.origin, db.policy, ceiling)
            .is_err());
        assert_eq!(db.keys(), before);
    }
    let challenge = db.challenge();
    for expires in [
        now / 1000 - 1,
        now / 1000 + 600,
        db.origin.valid_until_unix_seconds + 1,
        i64::MAX,
    ] {
        let mut changed = challenge.clone();
        changed.expires_at_unix_seconds = expires;
        assert!(db.store.create_certificate_challenge(&changed).is_err());
        assert_eq!(db.keys(), before);
    }
    let mut wrong = challenge;
    wrong.identity.auth_generation -= 1;
    assert!(db.store.create_certificate_challenge(&wrong).is_err());
    let mut wrong = db.identity.clone();
    wrong.principal.email = "other-owner@example.test".into();
    assert!(db
        .store
        .create_certificate_session(&wrong, &db.origin, db.policy, now + 60_000)
        .is_err());
    assert_eq!(db.keys(), before);
    assert!(db.snapshot(&sentinel) == sentinel_before);
}

#[test]
fn certificate_activity_requires_every_exact_origin_field_including_crl_at_the_same_revision() {
    let mut db = Fixture::new();
    db.identity.auth_generation = i64::MAX as u64;
    db.origin.auth_generation = db.identity.auth_generation;
    let ceiling = db.now() + 90_123;
    let (token, key, _) = db.issue(ceiling);
    db.age(&key, 10_000);
    let before = db.snapshot(&key);
    for case in 0..11 {
        let mut changed = db.origin.clone();
        match case {
            0 => changed.principal.id = UserId::new(),
            1 => changed.principal.email = "another-owner@example.test".into(),
            2 => changed.auth_generation -= 1,
            3 => changed.binding_id = uuid::Uuid::new_v4(),
            4 => changed.deployment_id = uuid::Uuid::new_v4(),
            5 => changed.trust_revision += 1,
            6 => changed.root_fingerprint = digest(41),
            7 => changed.leaf_fingerprint = digest(42),
            8 => changed.crl_digest = digest(43),
            9 => changed.valid_from_unix_seconds -= 1,
            _ => changed.valid_until_unix_seconds -= 1,
        }
        assert!(
            db.store
                .record_certificate_activity(&token, &db.identity, &changed, db.policy)
                .unwrap()
                .is_none(),
            "mismatched certificate provenance renewed a session"
        );
        assert!(
            db.snapshot(&key) == before,
            "rejected provenance changed a deadline or record"
        );
    }
    let mut neighbor = db.identity.clone();
    neighbor.auth_generation -= 1;
    assert!(db
        .store
        .record_certificate_activity(&token, &neighbor, &db.origin, db.policy)
        .unwrap()
        .is_none());
    assert!(db.snapshot(&key) == before);
    for policy in [
        SessionPolicy::new(120, None).unwrap(),
        SessionPolicy::new(120, Some(20)).unwrap(),
    ] {
        assert!(db.store.find_session(&token, policy).unwrap().is_none());
        assert!(db
            .store
            .record_certificate_activity(&token, &db.identity, &db.origin, policy)
            .unwrap()
            .is_none());
        assert!(db.snapshot(&key) == before);
    }
    let accepted = db
        .store
        .record_certificate_activity(&token, &db.identity, &db.origin, db.policy)
        .unwrap()
        .unwrap();
    assert_eq!(accepted.identity.auth_generation, i64::MAX as u64);
    assert_eq!(
        accepted.authentication,
        SessionAuthentication::Certificate(db.origin.clone().into())
    );
}

#[test]
fn malformed_certificate_session_origin_time_or_ttl_never_authenticates_or_repairs_state() {
    let mut db = Fixture::new();
    let ceiling = db.now() + 90_123;
    let (token, key, _) = db.issue(ceiling);
    let original = db.fields(&key);
    let expiration = db.deadline(&key);
    let auth: Value = serde_json::from_str(&original["authentication_json"]).unwrap();
    let mut edits = vec![
        ("version", "1".into()),
        ("version", "99".into()),
        ("unexpected", "1".into()),
        ("ceiling_unix_ms", (ceiling + 1).to_string()),
        ("ceiling_unix_ms", (MAX_EXACT + 1).to_string()),
        ("ceiling_unix_ms", format!("0{ceiling}")),
        ("last_activity_unix_ms", (db.now() + 60_000).to_string()),
        ("authentication_json", "null".into()),
        ("authentication_json", "[]".into()),
    ];
    for (pointer, value) in [
        ("/kind", json!("password")),
        ("/kind", json!("unknown")),
        ("/provenance/crl_digest", json!("broken")),
        (
            "/provenance/auth_generation",
            json!(db.identity.auth_generation - 1),
        ),
        ("/provenance/principal/id", json!(UserId::new())),
        (
            "/provenance/valid_from_unix_seconds",
            json!(db.now() / 1000 + 600),
        ),
        (
            "/provenance/valid_until_unix_seconds",
            json!(db.now() / 1000 - 1),
        ),
    ] {
        let mut changed = auth.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        edits.push(("authentication_json", changed.to_string()));
    }
    for pointer in ["", "/provenance", "/provenance/principal"] {
        let mut changed = auth.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unrecognized".into(), json!(true));
        edits.push(("authentication_json", changed.to_string()));
    }
    for (field, value) in edits {
        restore(&mut db, &key, &original, expiration);
        db.set(&key, field, value);
        rejected_unchanged(&mut db, &token, &key);
    }
    for field in ["authentication_json", "ceiling_unix_ms"] {
        restore(&mut db, &key, &original, expiration);
        db.connection.hdel::<_, _, ()>(&key, field).unwrap();
        rejected_unchanged(&mut db, &token, &key);
    }
    for expiry in [None, Some(expiration + 1), Some(expiration - 1)] {
        restore(&mut db, &key, &original, expiration);
        if let Some(at) = expiry {
            db.expire(&key, at);
        } else {
            db.connection.persist::<_, ()>(&key).unwrap();
        }
        rejected_unchanged(&mut db, &token, &key);
    }
}

fn restore(
    db: &mut Fixture,
    key: &str,
    fields: &std::collections::BTreeMap<String, String>,
    deadline: i64,
) {
    db.connection.del::<_, ()>(key).unwrap();
    for (field, value) in fields {
        db.set(key, field, value);
    }
    db.expire(key, deadline);
}

fn rejected_unchanged(db: &mut Fixture, token: &str, key: &str) {
    let before = db.snapshot(key);
    assert!(
        db.store.find_session(token, db.policy).unwrap().is_none(),
        "corrupt session authenticated"
    );
    assert!(db
        .store
        .record_activity(token, &db.identity, db.policy)
        .unwrap()
        .is_none());
    assert!(db
        .store
        .record_certificate_activity(token, &db.identity, &db.origin, db.policy)
        .unwrap()
        .is_none());
    assert!(
        db.snapshot(key) == before,
        "rejection repaired or changed corrupt state"
    );
}
