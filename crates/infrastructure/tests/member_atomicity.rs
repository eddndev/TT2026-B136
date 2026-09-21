mod member_support;
use application::members::{MemberError, MemberStore, UserAccessChange};
use application::ApplicationError;
use domain::identity::Role;
use member_support::{snapshot, store, user, Fixture};

#[test]
fn failed_access_audit_rolls_back_role_state_revision_generation_and_memberships() {
    let Some(mut db) = Fixture::new() else { return };
    let target = user(&mut db, "target@example.test", "litigator", true, true);
    let store = store(&db);
    store.get(db.owner, target, db.at).unwrap();
    db.admin.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_access CHECK(action <> 'identity.user_access_changed') NOT VALID").unwrap();
    let before = snapshot(&mut db);
    assert!(store
        .change_access(
            db.owner,
            target,
            UserAccessChange::new(0, Role::Paralegal, false).unwrap(),
            db.at
        )
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute("ALTER TABLE audit_events DROP CONSTRAINT reject_access")
        .unwrap();
    assert_eq!(
        store
            .change_access(
                db.owner,
                target,
                UserAccessChange::new(0, Role::Paralegal, false).unwrap(),
                db.at
            )
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn competing_owner_demotions_leave_an_active_owner() {
    let Some(mut db) = Fixture::new() else { return };
    let other = user(&mut db, "other@example.test", "owner", true, false);
    let first_store = store(&db);
    let second_store = store(&db);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let first_barrier = barrier.clone();
    let owner = db.owner;
    let at = db.at;
    let first = std::thread::spawn(move || {
        first_barrier.wait();
        first_store.change_access(
            owner,
            owner,
            UserAccessChange::new(0, Role::Paralegal, true).unwrap(),
            at,
        )
    });
    barrier.wait();
    let second = second_store.change_access(
        other,
        other,
        UserAccessChange::new(0, Role::Paralegal, true).unwrap(),
        at,
    );
    let results = [first.join().unwrap(), second];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(ApplicationError::Member(MemberError::LastActiveOwner))
            ))
            .count(),
        1
    );
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM users WHERE active AND role='owner'",
                &[]
            )
            .unwrap()
            .get::<_, i64>(0),
        1
    );
}
