use std::{sync::Arc, thread};

use application::ApplicationError;
use postgres::{Client, NoTls};
use uuid::Uuid;

use crate::{
    locking::{named_url, wait_for_lock, AUDIT_LOCK},
    support::*,
};

#[test]
fn current_receipt_is_owner_scoped_read_only_and_follows_withdrawal_and_renewal() {
    let Some((mut db, clock, trust)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let other = db.user("owner", false);
    let actor = principal(&mut db, owner);
    let other_actor = principal(&mut db, other);
    let repository = store(&db, &clock);
    let workflow = service(repository.clone(), &clock, actor);
    let other_workflow = service(repository.clone(), &clock, other_actor);
    let before = snapshot(&mut db);
    assert!(workflow.current_receipt(TOKEN).unwrap().is_none());
    assert!(other_workflow.current_receipt(TOKEN).unwrap().is_none());
    assert_eq!(snapshot(&mut db), before);

    let first_id = Uuid::from_u128(121);
    let (prepared, signature) = preparation(&workflow, first_id);
    let registered = workflow.register(TOKEN, &prepared, &signature).unwrap();
    let before = snapshot(&mut db);
    assert_eq!(
        repository.find_current(owner).unwrap(),
        Some(registered.clone())
    );
    assert!(other_workflow.current_receipt(TOKEN).unwrap().is_none());
    assert_eq!(snapshot(&mut db), before);

    clock.set(clock.at().unix_timestamp() + 1);
    let withdrawn = workflow.withdraw(TOKEN, first_id, 1).unwrap();
    let before = snapshot(&mut db);
    assert!(workflow.current_receipt(TOKEN).unwrap().is_none());
    assert!(other_workflow.current_receipt(TOKEN).unwrap().is_none());
    assert_eq!(
        workflow.receipt(TOKEN, first_id).unwrap(),
        Some(withdrawn.clone())
    );
    assert_eq!(snapshot(&mut db), before);

    clock.set(clock.at().unix_timestamp() + 1);
    let next_id = Uuid::from_u128(122);
    let (prepared, signature) = preparation(&workflow, next_id);
    let renewed = workflow.register(TOKEN, &prepared, &signature).unwrap();
    clock.set(trust.inspection.valid_until + 1);
    let before = snapshot(&mut db);
    assert_eq!(
        workflow.current_receipt(TOKEN).unwrap(),
        Some(renewed.clone())
    );
    assert!(other_workflow.current_receipt(TOKEN).unwrap().is_none());
    assert_eq!(
        workflow.receipt(TOKEN, first_id).unwrap(),
        Some(withdrawn.clone())
    );
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(counts(&mut db), (2, 1, 3));
    assert_audit(
        &db,
        &[(&registered, false), (&withdrawn, true), (&renewed, false)],
    );
}

#[test]
fn current_receipt_rechecks_active_owner_after_audit_wait_for_both_presence_and_absence() {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let other = db.user("owner", false);
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let workflow = service(repository, &clock, actor);
    let (prepared, signature) = preparation(&workflow, Uuid::from_u128(123));
    let registered = workflow.register(TOKEN, &prepared, &signature).unwrap();

    for target in [owner, other] {
        let actor = principal(&mut db, target);
        let (url, name) = named_url(&db.runtime_url, true);
        let waiting = Arc::new(PostgresOwnerCertificateStore::open(&url, clock.clone()).unwrap());
        let waiting_service = service(waiting, &clock, actor);
        let mut blocker = Client::connect(&db.admin_url, NoTls).unwrap();
        let mut tx = blocker.transaction().unwrap();
        tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
            .unwrap();
        let handle = thread::spawn(move || waiting_service.current_receipt(TOKEN));
        wait_for_lock(&mut db.admin, &name, "advisory");
        tx.execute(
            "UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&target.as_uuid()],
        ).unwrap();
        tx.commit().unwrap();
        let expected = snapshot(&mut db);
        assert!(matches!(
            handle.join().unwrap(),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(snapshot(&mut db), expected);
        if target == owner {
            db.admin.execute(
                "UPDATE users SET active=true,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
                &[&owner.as_uuid()],
            ).unwrap();
        }
    }
    assert_eq!(counts(&mut db), (1, 0, 1));
    assert_audit(&db, &[(&registered, false)]);
}
