use super::password_reset_backend_support::*;

#[test]
fn issuance_uses_server_time_and_an_independent_id_and_inspection_is_read_only() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    let issued = issue(&repository, target, 1);
    assert_eq!(issued.email, email(target));
    assert!(!issued.id.as_uuid().is_nil());
    assert_ne!(issued.id.as_uuid(), target.as_uuid());
    let row = db.admin.query_one(
        "SELECT digest,user_id,email,auth_generation,(extract(epoch FROM expires_at)*1000000)::bigint,
         extract(epoch FROM expires_at-issued_at)::bigint FROM password_reset_capabilities WHERE id=$1",
        &[&issued.id.as_uuid()],
    ).unwrap();
    assert_eq!(row.get::<_, Vec<u8>>(0), digest(1).as_bytes().to_vec());
    assert_eq!(row.get::<_, uuid::Uuid>(1), target.as_uuid());
    assert_eq!(row.get::<_, String>(2), email(target));
    assert_eq!(row.get::<_, i64>(3), 0);
    assert_eq!(
        i128::from(row.get::<_, i64>(4)) * 1000,
        issued.expires_at.unix_timestamp_nanos()
    );
    assert_eq!(row.get::<_, i64>(5), 60);
    let before = snapshot(&mut db);
    let observed = repository.inspect(digest(1)).unwrap().unwrap();
    assert_eq!(observed.id, issued.id);
    assert_eq!(observed.user_id, target);
    assert_eq!(observed.auth_generation, 0);
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn unknown_inactive_and_full_accounts_are_ignored_without_replacing_live_links() {
    let mut db = fixture();
    let target = account(&mut db, "paralegal", true);
    let inactive = account(&mut db, "client", false);
    let repository = store(&db);
    let first = issue(&repository, target, 1);
    let second = issue(&repository, target, 2);
    assert_ne!(first.id, second.id);
    let before = snapshot(&mut db);
    for input in [
        command(UserId::new(), 3, 2),
        command(inactive, 4, 2),
        command(target, 5, 2),
    ] {
        assert!(matches!(
            repository.issue(input).unwrap(),
            ResetIssueOutcome::Ignored
        ));
    }
    assert_eq!(snapshot(&mut db), before);
    assert!(repository.inspect(digest(1)).unwrap().is_some());
    assert!(repository.inspect(digest(2)).unwrap().is_some());
    expire(&mut db, first.id);
    assert!(repository.inspect(digest(1)).unwrap().is_none());
    assert!(matches!(
        repository.issue(command(target, 6, 2)).unwrap(),
        ResetIssueOutcome::Issued(_)
    ));
}

#[test]
fn duplicate_digest_never_overwrites_and_storage_errors_do_not_disclose_it() {
    let mut db = fixture();
    let target = account(&mut db, "owner", true);
    let repository = store(&db);
    issue(&repository, target, 0x73);
    let before = snapshot(&mut db);
    let error = repository.issue(command(target, 0x73, 2)).err().unwrap();
    assert!(!error.to_string().contains(&digest(0x73).to_hex()));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn delayed_cancellation_requires_both_the_exact_issuance_id_and_digest() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    let original = issue(&repository, target, 1);
    expire(&mut db, original.id);
    // Administrative removal simulates retirement of an old row; runtime has no cleanup API.
    db.admin
        .execute(
            "DELETE FROM password_reset_capabilities WHERE id=$1",
            &[&original.id.as_uuid()],
        )
        .unwrap();
    let replacement = issue(&repository, target, 1);
    assert_ne!(original.id, replacement.id);
    let before = snapshot(&mut db);
    repository
        .cancel_undelivered(original.id, digest(1))
        .unwrap();
    repository
        .cancel_undelivered(replacement.id, digest(2))
        .unwrap();
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        repository.inspect(digest(1)).unwrap().unwrap().id,
        replacement.id
    );
    repository
        .cancel_undelivered(replacement.id, digest(1))
        .unwrap();
    assert!(repository.inspect(digest(1)).unwrap().is_none());
    let cancelled = snapshot(&mut db);
    repository
        .cancel_undelivered(replacement.id, digest(1))
        .unwrap();
    assert_eq!(snapshot(&mut db), cancelled);
    assert_eq!(
        password_state(&mut db, target),
        ("original-hash".into(), 0, 0)
    );
}

#[test]
fn concurrent_issuance_cannot_exceed_the_account_generation_quota() {
    let mut db = fixture();
    let target = account(&mut db, "paralegal", true);
    let first = store(&db);
    let second = store(&db);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other = barrier.clone();
    let worker = std::thread::spawn(move || {
        other.wait();
        first.issue(command(target, 1, 1)).unwrap()
    });
    barrier.wait();
    let results = [
        second.issue(command(target, 2, 1)).unwrap(),
        worker.join().unwrap(),
    ];
    assert_eq!(
        results
            .iter()
            .filter(|value| matches!(value, ResetIssueOutcome::Issued(_)))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|value| matches!(value, ResetIssueOutcome::Ignored))
            .count(),
        1
    );
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM password_reset_capabilities", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
}
