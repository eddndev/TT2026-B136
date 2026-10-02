use super::password_reset_backend_support::*;

#[test]
fn consumption_for_each_role_preserves_current_mfa_identity_and_memberships() {
    let mut db = fixture();
    let repository = store(&db);
    for (index, role) in ["owner", "litigator", "paralegal", "client"]
        .iter()
        .enumerate()
    {
        let target = account(&mut db, role, true);
        let value = index as u8 + 1;
        let issued = issue(&repository, target, value);
        let observed = repository.inspect(digest(value)).unwrap().unwrap();
        db.admin.execute(
            "UPDATE users SET recovery_codes=jsonb_set(recovery_codes,'{slots,0}','null'),revision=revision+1 WHERE id=$1",
            &[&target.as_uuid()],
        ).unwrap();
        let preserved = stable_identity(&mut db, target);
        assert_eq!(
            repository
                .complete(complete(observed, value, "replacement-hash"))
                .unwrap(),
            ResetCompletion::Changed
        );
        assert_eq!(stable_identity(&mut db, target), preserved);
        assert_eq!(
            password_state(&mut db, target),
            ("replacement-hash".into(), 2, 1)
        );
        let row = db
            .admin
            .query_one(
                "SELECT r.consumed_revision,r.consumed_generation,a.actor,a.action,a.resource,
             a.timestamp::timestamptz=r.consumed_at FROM password_reset_capabilities r
             JOIN audit_events a ON a.sequence=r.audit_sequence WHERE r.id=$1",
                &[&issued.id.as_uuid()],
            )
            .unwrap();
        assert_eq!(row.get::<_, i64>(0), 2);
        assert_eq!(row.get::<_, i64>(1), 1);
        assert_eq!(row.get::<_, String>(2), "password-reset");
        assert_eq!(row.get::<_, String>(3), "identity.password_reset");
        assert_eq!(
            row.get::<_, String>(4),
            format!("user:{target}:revision:2:generation:1")
        );
        assert!(row.get::<_, bool>(5));
    }
    assert_eq!(reset_events(&mut db), 4);
    assert_chain(&db);
}

#[test]
fn one_consumption_rejects_replay_and_siblings_without_a_second_password_change() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    issue(&repository, target, 2);
    let original = repository.inspect(digest(1)).unwrap().unwrap();
    let sibling = repository.inspect(digest(2)).unwrap().unwrap();
    assert_eq!(
        repository
            .complete(complete(original.clone(), 1, "accepted-hash"))
            .unwrap(),
        ResetCompletion::Changed
    );
    let before = snapshot(&mut db);
    for (candidate, value) in [(original, 1), (sibling, 2)] {
        assert_eq!(
            repository
                .complete(complete(candidate, value, "replayed-hash"))
                .unwrap(),
            ResetCompletion::Rejected
        );
        assert!(repository.inspect(digest(value)).unwrap().is_none());
    }
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        password_state(&mut db, target),
        ("accepted-hash".into(), 1, 1)
    );
    assert_eq!(reset_events(&mut db), 1);
    assert_chain(&db);
}

#[test]
fn wrong_id_digest_account_generation_and_unchanged_hash_leave_all_state_unchanged() {
    let mut db = fixture();
    let target = account(&mut db, "paralegal", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    let original = repository.inspect(digest(1)).unwrap().unwrap();
    let mut wrong_id = original.clone();
    wrong_id.id = ResetId::from_uuid(uuid::Uuid::new_v4());
    let mut wrong_user = original.clone();
    wrong_user.user_id = db.owner;
    let mut wrong_generation = original.clone();
    wrong_generation.auth_generation += 1;
    let before = snapshot(&mut db);
    for input in [
        complete(wrong_id, 1, "new-hash"),
        complete(original.clone(), 2, "new-hash"),
        complete(wrong_user, 1, "new-hash"),
        complete(wrong_generation, 1, "new-hash"),
        complete(original, 1, "original-hash"),
    ] {
        assert_eq!(
            repository.complete(input).unwrap(),
            ResetCompletion::Rejected
        );
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn expiry_future_issue_cancellation_inactive_and_changed_generation_fail_closed() {
    for condition in ["expired", "future", "cancelled", "inactive", "generation"] {
        let mut db = fixture();
        let target = account(&mut db, "client", true);
        let repository = store(&db);
        let issued = issue(&repository, target, 1);
        let observed = repository.inspect(digest(1)).unwrap().unwrap();
        match condition {
            "expired" => expire(&mut db, issued.id),
            "future" => {
                db.admin.execute(
                "UPDATE password_reset_capabilities SET issued_at=clock_timestamp()+interval '1 minute',
                 expires_at=clock_timestamp()+interval '2 minutes' WHERE id=$1", &[&issued.id.as_uuid()],
            ).unwrap();
            }
            "cancelled" => repository.cancel_undelivered(issued.id, digest(1)).unwrap(),
            "inactive" => {
                db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&target.as_uuid()]).unwrap();
            }
            "generation" => {
                db.admin.execute("UPDATE users SET role='paralegal',revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&target.as_uuid()]).unwrap();
            }
            _ => unreachable!(),
        }
        let before = snapshot(&mut db);
        assert!(
            repository.inspect(digest(1)).unwrap().is_none(),
            "{condition}"
        );
        assert_eq!(
            repository
                .complete(complete(observed, 1, "rejected-hash"))
                .unwrap(),
            ResetCompletion::Rejected,
            "{condition}"
        );
        assert_eq!(snapshot(&mut db), before, "{condition}");
    }
}

#[test]
fn late_repeated_cancellation_preserves_a_confirmed_password_change_and_receipt() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    let issued = issue(&repository, target, 1);
    let candidate = repository.inspect(digest(1)).unwrap().unwrap();
    assert_eq!(
        repository
            .complete(complete(candidate, 1, "confirmed-hash"))
            .unwrap(),
        ResetCompletion::Changed
    );
    let confirmed = snapshot(&mut db);
    assert_eq!(reset_events(&mut db), 1);
    for _ in 0..2 {
        repository.cancel_undelivered(issued.id, digest(1)).unwrap();
        assert!(repository.inspect(digest(1)).unwrap().is_none());
        assert_eq!(snapshot(&mut db), confirmed);
        assert_eq!(reset_events(&mut db), 1);
        assert_eq!(
            password_state(&mut db, target),
            ("confirmed-hash".into(), 1, 1)
        );
        assert_chain(&db);
    }
}
