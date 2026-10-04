use super::support::*;

#[test]
fn certificate_sessions_bind_server_time_exact_origin_and_an_immutable_ceiling() {
    let mut db = Fixture::new();
    for idle in [None, Some(30)] {
        db.policy = SessionPolicy::new(120, idle).unwrap();
        for shortened in [false, true] {
            let before = db.now();
            let ceiling = before + if shortened { 20_123 } else { 240_123 };
            let (token, key, state) = db.issue(ceiling);
            let after = db.now();
            assert!((before..=after).contains(&state.server_now_unix_ms));
            assert_eq!(state.identity, db.identity);
            assert_eq!(
                state.authentication,
                SessionAuthentication::Certificate(db.origin.clone().into())
            );
            let absolute = (state.server_now_unix_ms + 120_000).min(ceiling);
            let expected_idle = idle
                .map(|seconds| (state.server_now_unix_ms + seconds as i64 * 1000).min(absolute));
            assert_eq!(state.absolute_expires_at_unix_ms, absolute);
            assert_eq!(state.idle_expires_at_unix_ms, expected_idle);
            let fields = db.fields(&key);
            assert_eq!(fields.len(), 10);
            assert_eq!(fields["version"], "2");
            assert_eq!(
                fields["identity_json"],
                serde_json::to_string(&db.identity).unwrap()
            );
            assert_eq!(
                serde_json::from_str::<Value>(&fields["authentication_json"]).unwrap(),
                authentication(&db.origin)
            );
            assert_eq!(number(&fields, "ceiling_unix_ms"), ceiling);
            assert_eq!(
                number(&fields, "created_at_unix_ms"),
                state.server_now_unix_ms
            );
            assert_eq!(number(&fields, "absolute_expires_at_unix_ms"), absolute);
            assert_eq!(db.deadline(&key), expected_idle.unwrap_or(absolute));
            assert!(
                !fields.values().any(|value| value.contains(&token)),
                "session value disclosed its bearer"
            );
            let snapshot = db.snapshot(&key);
            for _ in 0..3 {
                let found = db.store.find_session(&token, db.policy).unwrap().unwrap();
                assert_eq!(found.identity, db.identity);
                assert_eq!(found.authentication, state.authentication);
                assert_eq!(found.absolute_expires_at_unix_ms, absolute);
                assert_eq!(found.idle_expires_at_unix_ms, expected_idle);
                assert!(
                    db.snapshot(&key) == snapshot,
                    "read changed origin or expiration"
                );
            }
        }
    }
}

#[test]
fn explicit_certificate_activity_extends_only_idle_and_never_the_original_ceiling() {
    let mut db = Fixture::new();
    for capped in [false, true] {
        let ceiling = db.now() + if capped { 20_123 } else { 90_123 };
        let (token, key, _) = db.issue(ceiling);
        db.age(&key, 10_000);
        let mut before = db.fields(&key);
        let snapshot = db.snapshot(&key);
        assert!(db
            .store
            .record_activity(&token, &db.identity, db.policy)
            .unwrap()
            .is_none());
        assert!(
            db.snapshot(&key) == snapshot,
            "password activity renewed certificate state"
        );
        let updated = db
            .store
            .record_certificate_activity(&token, &db.identity, &db.origin, db.policy)
            .unwrap()
            .unwrap();
        let mut after = db.fields(&key);
        assert_eq!(
            updated.authentication,
            SessionAuthentication::Certificate(db.origin.clone().into())
        );
        assert_eq!(
            updated.absolute_expires_at_unix_ms,
            number(&before, "absolute_expires_at_unix_ms")
        );
        assert_eq!(
            updated.idle_expires_at_unix_ms,
            Some((updated.server_now_unix_ms + 30_000).min(updated.absolute_expires_at_unix_ms))
        );
        assert!(number(&after, "last_activity_unix_ms") > number(&before, "last_activity_unix_ms"));
        assert_eq!(db.deadline(&key), updated.idle_expires_at_unix_ms.unwrap());
        for field in ["last_activity_unix_ms", "idle_expires_at_unix_ms"] {
            before.remove(field);
            after.remove(field);
        }
        assert_eq!(after, before);
    }
    db.policy = SessionPolicy::new(120, None).unwrap();
    let ceiling = db.now() + 90_123;
    let (token, key, _) = db.issue(ceiling);
    db.age(&key, 10_000);
    let before = db.snapshot(&key);
    let state = db
        .store
        .record_certificate_activity(&token, &db.identity, &db.origin, db.policy)
        .unwrap()
        .unwrap();
    assert_eq!(state.idle_expires_at_unix_ms, None);
    assert!(
        db.snapshot(&key) == before,
        "absolute-only activity mutated the record"
    );
}
