use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use redis::Commands;

use super::{material, support::*};

#[test]
fn independent_clients_share_fixed_start_and_proof_windows_without_charging_on_denial() {
    let mut db = Fixture::new();
    let (owner, binding, start) = db.subject();
    let (_, other_binding, _) = db.subject();
    let another_start = db.reserve(start_key(owner, other_binding));
    let (third_owner, third_binding, third_start) = db.subject();
    let (first, _) = db.token();
    let (second, _) = db.token();
    let (third, _) = db.token();
    let store = db.store(policy(2, 1));
    store.admit_start(owner, binding).unwrap();
    let before = db.snapshot();
    exhausted(db.store(policy(2, 1)).admit_start(owner, binding));
    assert_eq!(db.snapshot(), before);
    store.admit_start(owner, other_binding).unwrap();
    let before = db.snapshot();
    exhausted(store.admit_start(third_owner, third_binding));
    assert_eq!(db.snapshot(), before);
    assert_eq!(db.fields(&start)["count"], "1");
    assert_eq!(db.fields(&another_start)["count"], "1");
    assert!(!db.connection.exists::<_, bool>(&third_start).unwrap());
    assert_eq!(db.fields(START_GLOBAL)["count"], "2");
    assert_eq!(db.fields(START_GLOBAL)["window_ms"], "60000");
    assert_eq!(db.fields(&start)["window_ms"], "30000");
    let starts = db.read(START_GLOBAL);
    store.admit_proof(&first).unwrap();
    let before = db.snapshot();
    exhausted(db.store(policy(2, 1)).admit_proof(&first));
    assert_eq!(db.snapshot(), before);
    store.admit_proof(&second).unwrap();
    let before = db.snapshot();
    exhausted(store.admit_proof(&third));
    assert_eq!(db.snapshot(), before);
    assert_eq!(db.read(START_GLOBAL), starts);
    assert_eq!(db.fields(PROOF_GLOBAL)["window_ms"], "90000");
    assert_eq!(db.fields(&proof_key(&first))["window_ms"], "45000");
}

fn burst(db: &Fixture, subjects: Vec<(UserId, Uuid)>) -> usize {
    let barrier = Arc::new(Barrier::new(subjects.len() + 1));
    let workers: Vec<_> = subjects
        .into_iter()
        .map(|(owner, binding)| {
            let store = db.store(policy(7, 3));
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                match store.admit_start(owner, binding) {
                    Ok(()) => true,
                    Err(ApplicationError::AccountLocked) => false,
                    Err(_) => panic!("atomic admission failed instead of reporting quota"),
                }
            })
        })
        .collect();
    barrier.wait();
    workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .filter(|admitted| *admitted)
        .count()
}

#[test]
fn concurrent_start_admissions_obey_subject_and_global_capacity_across_instances() {
    let mut db = Fixture::new();
    let (owner, binding, key) = db.subject();
    assert_eq!(burst(&db, vec![(owner, binding); 16]), 3);
    assert_eq!(db.fields(&key)["count"], "3");
    assert_eq!(db.fields(START_GLOBAL)["count"], "3");
    let others = (0..8)
        .map(|_| {
            let (owner, binding, _) = db.subject();
            (owner, binding)
        })
        .collect();
    assert_eq!(burst(&db, others), 4);
    assert_eq!(db.fields(START_GLOBAL)["count"], "7");
    assert_eq!(db.fields(&key)["count"], "3");
}

#[test]
fn expired_subject_budget_reopens_without_extending_global_or_consuming_other_controls() {
    let mut db = Fixture::new();
    let (owner, binding, key) = db.subject();
    let store = db.store(policy(4, 1));
    store.admit_start(owner, binding).unwrap();
    let global_deadline = db.deadline(START_GLOBAL);
    let deadline = db.now() + 150;
    db.connection
        .hset_multiple::<_, _, _, ()>(
            &key,
            &[
                ("opened_at_unix_ms", (deadline - 30_000).to_string()),
                ("expires_at_unix_ms", deadline.to_string()),
            ],
        )
        .unwrap();
    db.expire(&key, deadline);
    let before = db.snapshot();
    exhausted(store.admit_start(owner, binding));
    assert_eq!(db.snapshot(), before);
    let until = Instant::now() + Duration::from_secs(2);
    while db.connection.exists::<_, bool>(&key).unwrap() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !db.connection.exists::<_, bool>(&key).unwrap(),
        "fixture window did not expire"
    );
    store.admit_start(owner, binding).unwrap();
    assert_eq!(db.fields(&key)["count"], "1");
    assert_eq!(db.fields(START_GLOBAL)["count"], "2");
    assert_eq!(db.deadline(START_GLOBAL), global_deadline);
}

#[test]
fn proof_consumption_preserves_quotas_and_existing_password_reset_state() {
    let mut db = Fixture::new();
    let value = material::capture(db.now() / 1000, 90);
    let (token, _) = db.issue(&value, 90);
    let proof = db.reserve(proof_key(&token));
    let start = db.reserve(start_key(
        value.context.account.principal.id,
        value.context.binding_id,
    ));
    let reset = db.reserve("identity:password-reset:v1:request:global".into());
    let password = db.reserve(format!("identity:password-failures:{}", "a".repeat(64)));
    let until = db.now() + 60_000;
    for key in [&reset, &password] {
        db.string(key, "foreign-control-sentinel", until);
    }
    let reset_before = db.read(&reset);
    let password_before = db.read(&password);
    let store = db.store(policy(2, 1));
    store
        .admit_start(value.context.account.principal.id, value.context.binding_id)
        .unwrap();
    store.admit_proof(&token).unwrap();
    let keys = [START_GLOBAL, PROOF_GLOBAL, start.as_str(), proof.as_str()];
    let before: Vec<_> = keys.iter().map(|key| db.read(key)).collect();
    assert_eq!(store.take(&token).unwrap(), Some(value));
    for (key, prior) in keys.into_iter().zip(before) {
        assert_eq!(db.read(key), prior);
    }
    assert_eq!(db.read(&reset), reset_before);
    assert_eq!(db.read(&password), password_before);
}
