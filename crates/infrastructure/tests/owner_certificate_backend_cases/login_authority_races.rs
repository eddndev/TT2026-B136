use application::identity::certificate_login::OwnerLoginAuthority;
use postgres::{Client, NoTls};
use std::thread;
use uuid::Uuid;

use crate::{
    locking::{named_url, publish_locked, wait_for_lock, AUDIT_LOCK},
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

#[test]
fn login_authority_reads_exact_successor_crl_committed_while_it_waits() {
    let Some((mut db, clock, trust)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let workflow = service(repository.clone(), &clock, actor);
    let binding = Uuid::from_u128(147);
    let (prepared, signature) = preparation(&workflow, binding);
    let registered = workflow.register(TOKEN, &prepared, &signature).unwrap();
    let (url, name) = named_url(&db.runtime_url, true);
    let waiting = PostgresOwnerCertificateStore::open(&url, clock.clone()).unwrap();
    let mut blocker = Client::connect(&db.admin_url, NoTls).unwrap();
    let mut tx = blocker.transaction().unwrap();
    tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
        .unwrap();
    let handle = thread::spawn(move || waiting.load(owner, binding));
    wait_for_lock(&mut db.admin, &name, "advisory");
    clock.set(clock.at().unix_timestamp() + 1);
    publish_locked(&mut tx, &trust, &clock);
    tx.commit().unwrap();
    let expected = snapshot(&mut db);
    let expected_trust = repository.load_registration(owner).unwrap().trust.unwrap();

    let context = handle.join().unwrap().unwrap().unwrap();
    assert_eq!(context.binding_id, binding);
    assert_eq!(context.certificate, registered.check.certificate);
    assert_eq!(context.trust, expected_trust);
    assert_eq!(context.trust.revision, trust.revision.next().unwrap());
    assert_ne!(
        context.trust.inspection.crl_digest,
        trust.inspection.crl_digest
    );
    assert_eq!(
        context.trust.inspection.root_fingerprint,
        trust.inspection.root_fingerprint
    );
    assert_eq!(
        repository.find(owner, binding).unwrap(),
        Some(registered.clone())
    );
    assert_eq!(snapshot(&mut db), expected);
    assert_eq!(counts(&mut db), (1, 0, 1));
    assert_audit(&db, &[(&registered, false)]);
}
