use application::identity::{SessionPolicy, SessionStore};
use redis::Commands;

use super::redis_session_support::{number, Fixture};

#[test]
fn altered_session_metadata_never_authenticates_or_renews() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    let future = (fixture.now() + 60_000).to_string();
    for (field, value) in [
        ("version", "2"),
        ("unexpected", "1"),
        ("identity_json", "invalid-json"),
        ("absolute_ttl_ms", "0"),
        ("absolute_ttl_ms", "120001"),
        ("idle_ttl_ms", "0"),
        ("created_at_unix_ms", "1.5"),
        ("created_at_unix_ms", "01"),
        ("created_at_unix_ms", "9007199254740992"),
        ("created_at_unix_ms", future.as_str()),
        ("last_activity_unix_ms", "0"),
        ("last_activity_unix_ms", future.as_str()),
        ("absolute_expires_at_unix_ms", "0"),
        ("idle_expires_at_unix_ms", "0"),
        ("idle_expires_at_unix_ms", "9007199254740991"),
    ] {
        fixture.seed(10_000);
        fixture.set(field, value);
        let ttl_before: i64 = fixture.connection.pttl(&fixture.key).unwrap();
        assert!(
            fixture
                .store
                .find_session(&fixture.token, fixture.policy)
                .unwrap()
                .is_none(),
            "{field}={value}"
        );
        assert!(
            fixture
                .store
                .record_activity(&fixture.token, &fixture.identity, fixture.policy)
                .unwrap()
                .is_none(),
            "{field}={value}"
        );
        let ttl: i64 = fixture.connection.pttl(&fixture.key).unwrap();
        assert!(ttl == -2 || (0..=ttl_before).contains(&ttl));
    }
}

#[test]
fn each_stored_field_is_required() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    fixture.seed(0);
    let names: Vec<_> = fixture.raw().into_keys().collect();
    for field in names {
        fixture.seed(0);
        fixture
            .connection
            .hdel::<_, _, ()>(&fixture.key, &field)
            .unwrap();
        assert!(
            fixture
                .store
                .find_session(&fixture.token, fixture.policy)
                .unwrap()
                .is_none(),
            "{field}"
        );
        assert!(
            fixture
                .store
                .record_activity(&fixture.token, &fixture.identity, fixture.policy)
                .unwrap()
                .is_none(),
            "{field}"
        );
    }
}

#[test]
fn identity_json_rejects_unknown_fields_and_unrepresentable_generations() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    let canonical = serde_json::to_value(&fixture.identity).unwrap();
    let mut unknown_identity = canonical.clone();
    unknown_identity["unexpected"] = true.into();
    let mut unknown_principal = canonical.clone();
    unknown_principal["principal"]["unexpected"] = true.into();
    let mut overflowing = canonical;
    overflowing["auth_generation"] = u64::MAX.into();
    for identity in [unknown_identity, unknown_principal, overflowing] {
        fixture.seed(0);
        fixture.set("identity_json", identity.to_string());
        assert!(fixture
            .store
            .find_session(&fixture.token, fixture.policy)
            .unwrap()
            .is_none());
        assert!(fixture
            .store
            .record_activity(&fixture.token, &fixture.identity, fixture.policy)
            .unwrap()
            .is_none());
    }
}

#[test]
fn a_different_policy_requires_reauthentication_without_rewriting_the_session() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    fixture.seed(0);
    let before = fixture.raw();
    for policy in [
        SessionPolicy::new(120, None).unwrap(),
        SessionPolicy::new(120, Some(20)).unwrap(),
        SessionPolicy::new(60, Some(30)).unwrap(),
    ] {
        assert!(fixture
            .store
            .find_session(&fixture.token, policy)
            .unwrap()
            .is_none());
        assert!(fixture
            .store
            .record_activity(&fixture.token, &fixture.identity, policy)
            .unwrap()
            .is_none());
        assert_eq!(fixture.raw(), before);
    }
}

#[test]
fn incompatible_or_missing_redis_expiration_never_authenticates_or_is_repaired() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    for ttl in [None, Some(1_000), Some(90_000)] {
        fixture.seed(0);
        match ttl {
            Some(ttl) => fixture
                .connection
                .pexpire::<_, ()>(&fixture.key, ttl)
                .unwrap(),
            None => fixture.connection.persist::<_, ()>(&fixture.key).unwrap(),
        }
        let before: i64 = fixture.connection.pttl(&fixture.key).unwrap();
        assert!(fixture
            .store
            .find_session(&fixture.token, fixture.policy)
            .unwrap()
            .is_none());
        assert!(fixture
            .store
            .record_activity(&fixture.token, &fixture.identity, fixture.policy)
            .unwrap()
            .is_none());
        let after: i64 = fixture.connection.pttl(&fixture.key).unwrap();
        assert!(after == -2 || (before == -1 && after == -1) || (0..=before).contains(&after));
    }
    for offset_ms in [-1, 1] {
        fixture.seed(0);
        let deadline = number(&fixture.raw(), "idle_expires_at_unix_ms") + offset_ms;
        fixture.expire_at(deadline);
        assert!(fixture
            .store
            .find_session(&fixture.token, fixture.policy)
            .unwrap()
            .is_none());
        assert!(fixture
            .store
            .record_activity(&fixture.token, &fixture.identity, fixture.policy)
            .unwrap()
            .is_none());
        let remaining: i64 = redis::cmd("PEXPIRETIME")
            .arg(&fixture.key)
            .query(&mut fixture.connection)
            .unwrap();
        assert!(remaining == -2 || remaining == deadline);
    }
}

#[test]
fn non_hash_session_values_are_rejected_without_a_wrong_type_error() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    fixture
        .connection
        .rpush::<_, _, ()>(&fixture.key, "invalid")
        .unwrap();
    fixture
        .connection
        .expire::<_, ()>(&fixture.key, 30)
        .unwrap();
    assert!(fixture
        .store
        .find_session(&fixture.token, fixture.policy)
        .unwrap()
        .is_none());
    assert!(fixture
        .store
        .record_activity(&fixture.token, &fixture.identity, fixture.policy)
        .unwrap()
        .is_none());
}

#[test]
fn generations_above_lua_integer_precision_remain_exact_and_do_not_match_neighbors() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    for generation in [9_007_199_254_740_993, i64::MAX as u64] {
        fixture.identity.auth_generation = generation;
        fixture.seed(10_000);
        let before = fixture.raw();
        let found = fixture
            .store
            .find_session(&fixture.token, fixture.policy)
            .unwrap()
            .unwrap();
        assert_eq!(found.identity, fixture.identity);
        let mut wrong_identity = fixture.identity.clone();
        wrong_identity.auth_generation -= 1;
        assert!(fixture
            .store
            .record_activity(&fixture.token, &wrong_identity, fixture.policy)
            .unwrap()
            .is_none());
        assert_eq!(fixture.raw(), before);
        let updated = fixture
            .store
            .record_activity(&fixture.token, &fixture.identity, fixture.policy)
            .unwrap()
            .unwrap();
        assert_eq!(updated.identity.auth_generation, generation);
        assert_eq!(fixture.raw()["identity_json"], before["identity_json"]);
    }
}
