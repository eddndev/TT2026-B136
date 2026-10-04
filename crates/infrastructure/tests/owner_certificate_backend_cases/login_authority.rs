use application::identity::certificate_login::OwnerLoginAuthority;
use domain::identity::UserId;
use postgres::{Client, NoTls};
use uuid::Uuid;

use crate::{
    locking::{publish_locked, AUDIT_LOCK},
    support::*,
};

#[test]
fn login_authority_returns_exact_current_account_binding_and_trust_without_writing() {
    let Some((mut db, clock, trust)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let workflow = service(repository.clone(), &clock, actor.clone());
    let binding = Uuid::from_u128(141);
    let (prepared, signature) = preparation(&workflow, binding);
    let registered = workflow.register(TOKEN, &prepared, &signature).unwrap();
    let expected_account = repository.load_registration(owner).unwrap().account;
    let before = snapshot(&mut db);

    let context = repository.load(owner, binding).unwrap().unwrap();
    assert_eq!(context.account, expected_account);
    assert_eq!(context.account.principal, actor);
    assert_eq!(context.binding_id, binding);
    assert_eq!(context.certificate, registered.check.certificate);
    assert_eq!(context.trust, trust);
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(counts(&mut db), (1, 0, 1));
    assert_audit(&db, &[(&registered, false)]);
}

#[test]
fn login_authority_known_absence_never_substitutes_a_different_owner_or_binding() {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let other_owner = db.user("owner", false);
    let litigator = db.user("litigator", false);
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let workflow = service(repository.clone(), &clock, actor);
    let binding = Uuid::from_u128(142);
    let (prepared, signature) = preparation(&workflow, binding);
    let registered = workflow.register(TOKEN, &prepared, &signature).unwrap();
    let before = snapshot(&mut db);
    for (selected_owner, selected_binding) in [
        (owner, Uuid::from_u128(999)),
        (owner, Uuid::nil()),
        (other_owner, binding),
        (litigator, binding),
        (UserId::from_uuid(Uuid::nil()), binding),
    ] {
        assert!(repository
            .load(selected_owner, selected_binding)
            .unwrap()
            .is_none());
    }
    assert_eq!(snapshot(&mut db), before);

    db.admin
        .execute(
            "UPDATE users SET active=false,revision=revision+1,
        auth_generation=auth_generation+1 WHERE id=$1",
            &[&owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    assert!(repository.load(owner, binding).unwrap().is_none());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .execute(
            "UPDATE users SET active=true,revision=revision+1,
        auth_generation=auth_generation+1 WHERE id=$1",
            &[&owner.as_uuid()],
        )
        .unwrap();

    clock.set(clock.at().unix_timestamp() + 1);
    let withdrawn = workflow.withdraw(TOKEN, binding, 1).unwrap();
    let next_binding = Uuid::from_u128(143);
    let (prepared, signature) = preparation(&workflow, next_binding);
    let renewed = workflow.register(TOKEN, &prepared, &signature).unwrap();
    let before = snapshot(&mut db);
    assert!(repository.load(owner, binding).unwrap().is_none());
    assert_eq!(
        repository
            .load(owner, next_binding)
            .unwrap()
            .unwrap()
            .binding_id,
        next_binding
    );
    assert_eq!(
        repository.find(owner, binding).unwrap(),
        Some(withdrawn.clone())
    );
    assert_eq!(snapshot(&mut db), before);
    assert_audit(
        &db,
        &[(&registered, false), (&withdrawn, true), (&renewed, false)],
    );
}

#[test]
fn login_authority_returns_new_account_and_crl_facts_without_rewriting_historical_receipt() {
    let Some((mut db, clock, trust)) = fixture() else {
        return;
    };
    let owner = db.owner;
    db.user("owner", false);
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let workflow = service(repository.clone(), &clock, actor);
    let binding = Uuid::from_u128(144);
    let (prepared, signature) = preparation(&workflow, binding);
    let registered = workflow.register(TOKEN, &prepared, &signature).unwrap();
    let original = repository.load(owner, binding).unwrap().unwrap();
    clock.set(clock.at().unix_timestamp() + 1);
    let mut tx = db.admin.transaction().unwrap();
    tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
        .unwrap();
    // Access revocation and restoration advance both counters legally. A bare
    // generation increment without a credential/access change must be rejected.
    tx.execute(
        "UPDATE users SET active=false,email='updated-owner@example.test',
        revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
        &[&owner.as_uuid()],
    )
    .unwrap();
    tx.execute(
        "UPDATE users SET active=true,revision=revision+1,
        auth_generation=auth_generation+1 WHERE id=$1",
        &[&owner.as_uuid()],
    )
    .unwrap();
    publish_locked(&mut tx, &trust, &clock);
    tx.commit().unwrap();
    let before = snapshot(&mut db);

    let current = repository.load(owner, binding).unwrap().unwrap();
    assert_eq!(current.account.principal.id, owner);
    assert_eq!(
        current.account.principal.email,
        "updated-owner@example.test"
    );
    assert_eq!(current.account.revision, original.account.revision + 2);
    assert_eq!(
        current.account.auth_generation,
        original.account.auth_generation + 2
    );
    assert_eq!(current.trust.revision, trust.revision.next().unwrap());
    assert_ne!(
        current.trust.inspection.crl_digest,
        trust.inspection.crl_digest
    );
    assert_eq!(current.certificate, registered.check.certificate);
    assert_eq!(
        repository.find(owner, binding).unwrap(),
        Some(registered.clone())
    );
    assert_eq!(snapshot(&mut db), before);

    // Authority returns facts; application admission evaluates current time.
    clock.set(trust.inspection.valid_until + 1);
    assert_eq!(repository.load(owner, binding).unwrap(), Some(current));
    assert_eq!(repository.find(owner, binding).unwrap(), Some(registered));
    assert_eq!(snapshot(&mut db), before);
}

struct SelectGrant {
    client: Client,
    role: String,
}

impl SelectGrant {
    fn revoke(db: &Fixture) -> Self {
        let mut grant = Self {
            client: Client::connect(&db.admin_url, NoTls).unwrap(),
            role: db.role.clone(),
        };
        grant
            .client
            .batch_execute(&format!(
                "REVOKE SELECT ON owner_certificate_registrations FROM {}",
                grant.role
            ))
            .unwrap();
        grant
    }
}

impl Drop for SelectGrant {
    fn drop(&mut self) {
        let _ = self.client.batch_execute(&format!(
            "GRANT SELECT ON owner_certificate_registrations TO {}",
            self.role
        ));
    }
}

#[test]
fn login_authority_database_failure_is_an_error_not_absence_or_cached_authority() {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let workflow = service(repository.clone(), &clock, actor);
    let binding = Uuid::from_u128(145);
    let (prepared, signature) = preparation(&workflow, binding);
    workflow.register(TOKEN, &prepared, &signature).unwrap();
    let admitted = repository.load(owner, binding).unwrap().unwrap();
    let before = snapshot(&mut db);
    let grant = SelectGrant::revoke(&db);
    let permitted: bool = db
        .runtime()
        .query_one(
            "SELECT has_table_privilege(current_user,'owner_certificate_registrations','SELECT')",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(!permitted);
    let failed = repository.load(owner, binding);
    drop(grant);

    assert!(
        failed.is_err(),
        "a failed authority query must not look like absence"
    );
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(repository.load(owner, binding).unwrap(), Some(admitted));
    assert_eq!(snapshot(&mut db), before);
}
