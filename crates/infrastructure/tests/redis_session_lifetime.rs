use application::identity::SessionStore;
use redis::Commands;

use super::redis_session_support::{number, session_key, Fixture};

#[test]
fn creation_stores_a_versioned_identity_and_server_deadlines_with_one_expiration() {
    for idle in [None, Some(30)] {
        let Some(mut fixture) = Fixture::new(idle) else {
            return;
        };
        let before = fixture.now();
        let grant = fixture
            .store
            .create_session(&fixture.identity, fixture.policy)
            .unwrap();
        fixture.token = grant.access_token;
        fixture.key = session_key(&fixture.token);
        let after = fixture.now();
        let kind: String = redis::cmd("TYPE")
            .arg(&fixture.key)
            .query(&mut fixture.connection)
            .unwrap();
        assert_eq!(kind, "hash");
        let fields = fixture.raw();
        assert_eq!(fields.len(), 8);
        assert_eq!(fields["version"], "1");
        assert_eq!(
            fields["identity_json"],
            serde_json::to_string(&fixture.identity).unwrap()
        );
        assert!(!fields.values().any(|value| value.contains(&fixture.token)));
        let created = number(&fields, "created_at_unix_ms");
        assert!((before..=after).contains(&created));
        assert_eq!(number(&fields, "last_activity_unix_ms"), created);
        assert_eq!(number(&fields, "absolute_ttl_ms"), 120_000);
        assert_eq!(
            number(&fields, "idle_ttl_ms"),
            idle.unwrap_or(0) as i64 * 1_000
        );
        assert_eq!(
            number(&fields, "absolute_expires_at_unix_ms"),
            created + 120_000
        );
        assert_eq!(grant.state.server_now_unix_ms, created);
        assert_eq!(grant.state.identity, fixture.identity);
        assert_eq!(grant.state.absolute_expires_at_unix_ms, created + 120_000);
        assert_eq!(
            grant.state.idle_expires_at_unix_ms,
            idle.map(|ttl| created + ttl as i64 * 1_000)
        );
        let ttl: i64 = fixture.connection.pttl(&fixture.key).unwrap();
        let deadline = created + idle.unwrap_or(120) as i64 * 1_000;
        assert!((deadline - fixture.now()..=deadline - before).contains(&ttl));
    }
}

#[test]
fn reading_a_live_session_preserves_every_stored_field_and_does_not_extend_ttl() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    fixture.seed(10_000);
    let before = fixture.raw();
    let ttl_before: i64 = fixture.connection.pttl(&fixture.key).unwrap();
    for _ in 0..3 {
        let state = fixture
            .store
            .find_session(&fixture.token, fixture.policy)
            .unwrap()
            .unwrap();
        assert_eq!(state.identity, fixture.identity);
        assert_eq!(
            state.absolute_expires_at_unix_ms,
            number(&before, "absolute_expires_at_unix_ms")
        );
        assert_eq!(
            state.idle_expires_at_unix_ms,
            Some(number(&before, "idle_expires_at_unix_ms"))
        );
        assert_eq!(fixture.raw(), before);
        let ttl: i64 = fixture.connection.pttl(&fixture.key).unwrap();
        assert!((1..=ttl_before).contains(&ttl));
    }
}

#[test]
fn explicit_activity_updates_only_activity_idle_and_expiration() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    fixture.seed(10_000);
    let mut before = fixture.raw();
    let state = fixture
        .store
        .record_activity(&fixture.token, &fixture.identity, fixture.policy)
        .unwrap()
        .unwrap();
    let mut after = fixture.raw();
    assert_eq!(state.identity, fixture.identity);
    assert!(number(&after, "last_activity_unix_ms") > number(&before, "last_activity_unix_ms"));
    assert!(number(&after, "idle_expires_at_unix_ms") > number(&before, "idle_expires_at_unix_ms"));
    assert_eq!(
        state.server_now_unix_ms,
        number(&after, "last_activity_unix_ms")
    );
    assert_eq!(
        state.idle_expires_at_unix_ms,
        Some(state.server_now_unix_ms + 30_000)
    );
    let ttl: i64 = fixture.connection.pttl(&fixture.key).unwrap();
    assert!((20_001..=30_000).contains(&ttl));
    for field in ["last_activity_unix_ms", "idle_expires_at_unix_ms"] {
        before.remove(field);
        after.remove(field);
    }
    assert_eq!(before, after);
}

#[test]
fn activity_is_capped_by_the_original_absolute_deadline() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    fixture.seed(0);
    let now = fixture.now();
    let absolute = now + 20_000;
    // The stored activity keeps this old session live near its absolute limit.
    fixture.set("created_at_unix_ms", absolute - 120_000);
    fixture.set("absolute_expires_at_unix_ms", absolute);
    fixture.set("last_activity_unix_ms", now - 10_000);
    fixture.set("idle_expires_at_unix_ms", absolute);
    fixture.expire_at(absolute);
    let state = fixture
        .store
        .record_activity(&fixture.token, &fixture.identity, fixture.policy)
        .unwrap()
        .unwrap();
    assert_eq!(state.absolute_expires_at_unix_ms, absolute);
    assert_eq!(state.idle_expires_at_unix_ms, Some(absolute));
    let ttl: i64 = fixture.connection.pttl(&fixture.key).unwrap();
    assert!((1..=20_000).contains(&ttl));
}

#[test]
fn absolute_only_activity_keeps_the_initial_expiration() {
    let Some(mut fixture) = Fixture::new(None) else {
        return;
    };
    fixture.seed(10_000);
    let before = fixture.raw();
    let ttl_before: i64 = fixture.connection.pttl(&fixture.key).unwrap();
    let state = fixture
        .store
        .record_activity(&fixture.token, &fixture.identity, fixture.policy)
        .unwrap()
        .unwrap();
    assert_eq!(state.idle_expires_at_unix_ms, None);
    assert_eq!(
        state.absolute_expires_at_unix_ms,
        number(&before, "absolute_expires_at_unix_ms")
    );
    assert_eq!(fixture.raw(), before);
    let ttl: i64 = fixture.connection.pttl(&fixture.key).unwrap();
    assert!((1..=ttl_before).contains(&ttl));
}

#[test]
fn activity_on_absent_expired_or_revoked_sessions_never_recreates_a_key() {
    let Some(mut fixture) = Fixture::new(Some(30)) else {
        return;
    };
    for reason in ["absent", "expired", "revoked"] {
        if reason != "absent" {
            fixture.seed(0);
            if reason == "expired" {
                let past = fixture.now() - 1;
                fixture.expire_at(past);
            } else {
                fixture.store.revoke_session(&fixture.token).unwrap();
            }
        }
        assert!(
            fixture
                .store
                .record_activity(&fixture.token, &fixture.identity, fixture.policy)
                .unwrap()
                .is_none(),
            "{reason}"
        );
        assert!(fixture
            .store
            .find_session(&fixture.token, fixture.policy)
            .unwrap()
            .is_none());
        assert!(!fixture.exists(), "{reason}");
    }
}
