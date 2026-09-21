mod member_support;
use application::{
    members::{MemberError, MemberStore, UserAccessChange},
    ApplicationError,
};
use domain::identity::Role;
use member_support::{store, user, Fixture};

#[test]
fn direct_writes_cannot_remove_last_owner_or_rewrite_generation() {
    let Some(mut db) = Fixture::new() else { return };
    let mut runtime = db.runtime();
    for sql in [
        "UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE role='owner'",
        "UPDATE users SET auth_generation=auth_generation+1",
        "DELETE FROM users WHERE false",
        "UPDATE users SET password_hash='changed' WHERE false",
    ] {
        assert!(runtime.batch_execute(sql).is_err(), "unexpectedly allowed {sql}");
    }
    let target = user(&mut db, "bounded@example.test", "paralegal", true, false);
    db.admin
        .batch_execute("ALTER TABLE users DISABLE TRIGGER USER")
        .unwrap();
    db.admin
        .execute(
            "UPDATE users SET revision=$2,auth_generation=$2 WHERE id=$1",
            &[&target.as_uuid(), &i64::MAX],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE users ENABLE TRIGGER USER")
        .unwrap();
    let store = store(&db);
    store
        .change_access(
            db.owner,
            target,
            UserAccessChange::new(i64::MAX as u64, Role::Paralegal, true).unwrap(),
            db.at,
        )
        .unwrap();
    assert!(matches!(
        store.change_access(
            db.owner,
            target,
            UserAccessChange::new(i64::MAX as u64, Role::Client, true).unwrap(),
            db.at
        ),
        Err(ApplicationError::Member(
            MemberError::AccessVersionExhausted
        ))
    ));
}
