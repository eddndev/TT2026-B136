use std::{sync::Arc, thread};

use application::ApplicationError;
use domain::crypto::CredentialFailure;
use postgres::{Client, NoTls};
use uuid::Uuid;

use crate::{
    capture,
    locking::{named_url, publish_locked, wait_for_lock, AUDIT_LOCK},
    support::*,
};

#[test]
fn registration_reloads_authority_counters_and_current_trust_after_the_audit_lock_wait() {
    for change in 0..3 {
        let Some((mut db, clock, trust)) = fixture() else {
            return;
        };
        let owner = db.owner;
        db.user("owner", false);
        let actor = principal(&mut db, owner);
        let repository = store(&db, &clock);
        let command = capture::registration(repository, &clock, actor, Uuid::from_u128(71));
        let (url, name) = named_url(&db.runtime_url, false);
        let waiting = PostgresOwnerCertificateStore::open(&url, clock.clone()).unwrap();
        let mut blocker = Client::connect(&db.admin_url, NoTls).unwrap();
        let mut tx = blocker.transaction().unwrap();
        tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
            .unwrap();
        let handle = thread::spawn(move || waiting.commit_registration(command));
        wait_for_lock(&mut db.admin, &name, "advisory");
        match change {
            0 => {
                tx.execute(
                    "UPDATE users SET revision=revision+1 WHERE id=$1",
                    &[&owner.as_uuid()],
                )
                .unwrap();
            }
            1 => publish_locked(&mut tx, &trust, &clock),
            _ => {
                tx.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
                &[&owner.as_uuid()]).unwrap();
            }
        }
        tx.commit().unwrap();
        let result = handle.join().unwrap();
        match change {
            0 => category(result, OwnerCertificateError::AccountChanged),
            1 => category(result, OwnerCertificateError::TrustChanged),
            _ => assert!(matches!(result, Err(ApplicationError::PermissionDenied))),
        }
        assert_eq!(counts(&mut db), (0, 0, 0));
        let audits: i64 = db
            .admin
            .query_one("SELECT count(*) FROM audit_events", &[])
            .unwrap()
            .get(0);
        assert_eq!(audits, if change == 1 { 2 } else { 1 });
        assert_audit(&db, &[]);
    }
}

#[test]
fn registration_checks_expiry_after_the_owner_row_wait_and_overrides_repeatable_read() {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let command = capture::registration(
        repository.clone(),
        &clock,
        actor.clone(),
        Uuid::from_u128(81),
    );
    let expires = command.check().valid_until;
    let original_time = clock.at().unix_timestamp();
    let (url, name) = named_url(&db.runtime_url, true);
    let waiting = Arc::new(PostgresOwnerCertificateStore::open(&url, clock.clone()).unwrap());
    let worker = waiting.clone();
    let mut blocker = Client::connect(&db.admin_url, NoTls).unwrap();
    let mut tx = blocker.transaction().unwrap();
    // A read-only row lock deliberately tests the wait after audit admission.
    tx.query_one(
        "SELECT id FROM users WHERE id=$1 FOR UPDATE",
        &[&owner.as_uuid()],
    )
    .unwrap();
    let before = snapshot(&mut db);
    let handle = thread::spawn(move || worker.commit_registration(command));
    wait_for_lock(&mut db.admin, &name, "transactionid");
    clock.set(expires + 1);
    tx.rollback().unwrap();
    category(
        handle.join().unwrap(),
        OwnerCertificateError::Credential(CredentialFailure::Expired),
    );
    assert_eq!(snapshot(&mut db), before);
    clock.set(original_time);
    let fresh = capture::registration(repository, &clock, actor, Uuid::from_u128(82));
    let registered = applied(waiting.commit_registration(fresh).unwrap());
    assert_eq!(counts(&mut db), (1, 0, 1));
    assert_audit(&db, &[(&registered, false)]);
}
