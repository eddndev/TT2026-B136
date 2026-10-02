use super::password_reset_backend_support::*;
use time::format_description::well_known::Rfc3339;

#[test]
fn audit_failure_rolls_back_password_counters_consumption_and_preserves_retry() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    let observed = repository.inspect(digest(1)).unwrap().unwrap();
    db.admin.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_reset CHECK(action<>'identity.password_reset') NOT VALID").unwrap();
    let before = snapshot(&mut db);
    assert!(repository
        .complete(complete(observed.clone(), 1, "new-hash"))
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute("ALTER TABLE audit_events DROP CONSTRAINT reject_reset")
        .unwrap();
    assert_eq!(
        repository
            .complete(complete(observed, 1, "new-hash"))
            .unwrap(),
        ResetCompletion::Changed
    );
    assert_eq!(reset_events(&mut db), 1);
    assert_chain(&db);
}

#[test]
fn direct_consume_commit_requires_a_receipt_with_exact_actor_action_resource_and_time() {
    let mut db = fixture();
    let target = account(&mut db, "paralegal", true);
    let repository = store(&db);
    let issued = issue(&repository, target, 1);
    let query = raw_consume_sql(&db);
    for invalid in ["absent", "actor", "action", "resource", "time"] {
        let before = snapshot(&mut db);
        let mut runtime = db.runtime();
        let mut transaction = runtime.transaction().unwrap();
        let row = transaction
            .query_one(
                &query,
                &[
                    &issued.id.as_uuid(),
                    &&digest(1).as_bytes()[..],
                    &target.as_uuid(),
                    &0i64,
                    &"new-hash",
                ],
            )
            .unwrap();
        if invalid != "absent" {
            let micros: i64 = row.get("reset_at_micros");
            let mut at =
                time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(micros) * 1000).unwrap();
            if invalid == "time" {
                at += time::Duration::seconds(1);
            }
            let actor = if invalid == "actor" {
                "other"
            } else {
                "password-reset"
            };
            let action = if invalid == "action" {
                "identity.other"
            } else {
                "identity.password_reset"
            };
            let resource = if invalid == "resource" {
                "other".into()
            } else {
                format!("user:{target}:revision:1:generation:1")
            };
            let sequence: i64 = row.get("next_audit");
            transaction.execute(
                "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
                &[&sequence, &at.format(&Rfc3339).unwrap(), &actor, &action, &resource, &vec![0u8; 32]],
            ).unwrap();
        }
        assert!(transaction.commit().is_err(), "accepted {invalid} receipt");
        assert_eq!(snapshot(&mut db), before, "{invalid}");
    }
}

#[test]
fn competing_consumers_of_one_or_sibling_capabilities_commit_exactly_once() {
    for siblings in [false, true] {
        let mut db = fixture();
        let target = account(&mut db, "litigator", true);
        let first = store(&db);
        let second = store(&db);
        issue(&first, target, 1);
        if siblings {
            issue(&first, target, 2);
        }
        let one = first.inspect(digest(1)).unwrap().unwrap();
        let value = if siblings { 2 } else { 1 };
        let two = second.inspect(digest(value)).unwrap().unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let other = barrier.clone();
        let worker = std::thread::spawn(move || {
            other.wait();
            first.complete(complete(one, 1, "first-hash")).unwrap()
        });
        barrier.wait();
        let results = [
            second
                .complete(complete(two, value, "second-hash"))
                .unwrap(),
            worker.join().unwrap(),
        ];
        assert_eq!(
            results
                .iter()
                .filter(|value| **value == ResetCompletion::Changed)
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|value| **value == ResetCompletion::Rejected)
                .count(),
            1
        );
        let (hash, revision, generation) = password_state(&mut db, target);
        assert!(["first-hash", "second-hash"].contains(&hash.as_str()));
        assert_eq!((revision, generation), (1, 1));
        assert_eq!(reset_events(&mut db), 1);
        assert_chain(&db);
    }
}

#[test]
fn dropping_an_uncommitted_direct_consumption_preserves_the_capability() {
    let mut db = fixture();
    let target = account(&mut db, "owner", true);
    let repository = store(&db);
    let issued = issue(&repository, target, 1);
    let before = snapshot(&mut db);
    {
        let mut runtime = db.runtime();
        let mut transaction = runtime.transaction().unwrap();
        transaction
            .query_one(
                &raw_consume_sql(&db),
                &[
                    &issued.id.as_uuid(),
                    &&digest(1).as_bytes()[..],
                    &target.as_uuid(),
                    &0i64,
                    &"abandoned-hash",
                ],
            )
            .unwrap();
    }
    assert_eq!(snapshot(&mut db), before);
    assert!(repository.inspect(digest(1)).unwrap().is_some());
}
