use redis::Commands;

use super::support::*;

fn corrupt(db: &mut Fixture, key: &str, condition: &str) {
    match condition {
        "persistent" => {
            db.connection.persist::<_, ()>(key).unwrap();
        }
        "type" => {
            db.connection.del::<_, ()>(key).unwrap();
            db.connection
                .lpush::<_, _, ()>(key, "private-invalid-budget-marker")
                .unwrap();
        }
        "extra" => {
            db.connection
                .hset::<_, _, _, ()>(key, "unexpected", 1)
                .unwrap();
        }
        "missing" => {
            db.connection.hdel::<_, _, ()>(key, "version").unwrap();
        }
        "deadline" => {
            let deadline = db.deadline(key);
            db.expire(key, deadline - 1);
        }
        "version" => {
            db.connection
                .hset::<_, _, _, ()>(key, "version", 2)
                .unwrap();
        }
        "policy" => {
            db.connection
                .hset::<_, _, _, ()>(key, "window_ms", 1000)
                .unwrap();
        }
        "future" => {
            let window: i64 = db.fields(key)["window_ms"].parse().unwrap();
            let opened = db.now() + 10_000;
            db.connection
                .hset_multiple::<_, _, _, ()>(
                    key,
                    &[
                        ("opened_at_unix_ms", opened.to_string()),
                        ("expires_at_unix_ms", (opened + window).to_string()),
                    ],
                )
                .unwrap();
            db.expire(key, opened + window);
        }
        value => {
            db.connection
                .hset::<_, _, _, ()>(key, "count", value)
                .unwrap();
        }
    }
}

#[test]
fn corrupt_budget_fails_closed_before_writes_even_if_the_other_budget_is_full() {
    for proof in [false, true] {
        for global in [false, true] {
            for condition in [
                "0",
                "01",
                "2",
                "9007199254740992",
                "persistent",
                "type",
                "extra",
                "missing",
                "deadline",
                "version",
                "policy",
                "future",
            ] {
                let mut db = Fixture::new();
                let (owner, binding, start) = db.subject();
                let (token, _) = db.token();
                let proof_subject = proof_key(&token);
                let (global_key, subject) = if proof {
                    (PROOF_GLOBAL, proof_subject.as_str())
                } else {
                    (START_GLOBAL, start.as_str())
                };
                let store = db.store(policy(1, 1));
                let admit = || {
                    if proof {
                        store.admit_proof(&token)
                    } else {
                        store.admit_start(owner, binding)
                    }
                };
                admit().unwrap();
                let key = if global { global_key } else { subject };
                corrupt(&mut db, key, condition);
                let before = db.snapshot();
                let error = admit().expect_err("corrupt budget must not be admitted");
                assert!(
                    matches!(error, ApplicationError::Port(_)),
                    "corruption was treated as ordinary quota"
                );
                assert!(!error.to_string().contains("private-invalid-budget-marker"));
                assert!(!error.to_string().contains(&token));
                assert_eq!(
                    db.snapshot(),
                    before,
                    "corrupt budget was repaired or changed"
                );
            }
        }
    }
}

#[test]
fn changing_live_policy_cannot_reset_or_silently_reinterpret_existing_counters() {
    let mut db = Fixture::new();
    let (owner, binding, _) = db.subject();
    db.store(policy(2, 1)).admit_start(owner, binding).unwrap();
    let before = db.snapshot();
    for changed in [
        OwnerLoginRatePolicy::new(limit(3, 60), limit(1, 30), limit(2, 90), limit(1, 45)),
        OwnerLoginRatePolicy::new(limit(2, 61), limit(1, 30), limit(2, 90), limit(1, 45)),
    ] {
        assert!(matches!(
            db.store(changed).admit_start(owner, binding),
            Err(ApplicationError::Port(_))
        ));
        assert_eq!(db.snapshot(), before);
    }
}
