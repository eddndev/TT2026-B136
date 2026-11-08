use application::identity::certificate_login::OwnerLoginAuthority;
use postgres::{Client, NoTls};
use std::thread;
use uuid::Uuid;

use crate::{
    locking::{named_url, wait_for_lock, AUDIT_LOCK},
    support::*,
};

#[test]
fn login_authority_reloads_account_after_the_audit_wait_even_with_repeatable_read_default() {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    let owner = db.owner;
    db.user("owner", false);
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let workflow = service(repository, &clock, actor);
    let binding = Uuid::from_u128(146);
    let (prepared, signature) = preparation(&workflow, binding);
    let registered = workflow.register(TOKEN, &prepared, &signature).unwrap();

    for deactivate in [false, true] {
        let (url, name) = named_url(&db.runtime_url, true);
        let waiting = PostgresOwnerCertificateStore::open(&url, clock.clone()).unwrap();
        let mut blocker = Client::connect(&db.admin_url, NoTls).unwrap();
        let mut tx = blocker.transaction().unwrap();
        tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
            .unwrap();
        let handle = thread::spawn(move || waiting.load(owner, binding));
        wait_for_lock(&mut db.admin, &name, "advisory");
        if deactivate {
            tx.execute(
                "UPDATE users SET active=false,revision=revision+1,
                auth_generation=auth_generation+1 WHERE id=$1",
                &[&owner.as_uuid()],
            )
            .unwrap();
        } else {
            tx.execute(
                "UPDATE users SET role='litigator',revision=revision+1,
                auth_generation=auth_generation+1 WHERE id=$1",
                &[&owner.as_uuid()],
            )
            .unwrap();
        }
        tx.commit().unwrap();
        let expected = snapshot(&mut db);
        assert!(handle.join().unwrap().unwrap().is_none());
        assert_eq!(snapshot(&mut db), expected);
        db.admin
            .execute(
                "UPDATE users SET role='owner',active=true,revision=revision+1,
            auth_generation=auth_generation+1 WHERE id=$1",
                &[&owner.as_uuid()],
            )
            .unwrap();
    }
    assert_eq!(counts(&mut db), (1, 0, 1));
    assert_audit(&db, &[(&registered, false)]);
}
