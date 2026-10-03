use application::ApplicationError;
use domain::crypto::{CredentialFailure, DocumentHasher};
use infrastructure::RingSha256Hasher;
use uuid::Uuid;

use crate::{capture, owner_binding_fixture, support::*};

#[test]
fn registration_renewal_and_terminal_receipts_preserve_public_bytes_and_all_user_state() {
    let Some((mut db, clock, trust)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let original_users = users(&mut db);
    let repository = store(&db, &clock);
    let workflow = service(repository.clone(), &clock, actor);
    // A non-v4 non-nil identifier is an intentional accepted domain value.
    let first_id = Uuid::from_u128(17);
    let (prepared, signature) = preparation(&workflow, first_id);
    let registered = workflow.register(TOKEN, &prepared, &signature).unwrap();
    assert_eq!(registered.owner, owner);
    assert_eq!(registered.record.registration(), prepared.statement());
    assert_eq!(
        registered.check.certificate.der,
        owner_binding_fixture::fixture().leaf_der
    );
    assert_eq!(registered.check.signature, signature);
    assert_eq!(
        registered.check.statement_digest,
        RingSha256Hasher.hash_bytes(&prepared.statement().canonical_bytes())
    );
    assert_eq!(registered.trust, trust);
    assert_eq!(registered.registered_at, clock.at());
    assert_eq!(registered.withdrawn_at, None);
    assert_eq!(
        store(&db, &clock).find(owner, first_id).unwrap(),
        Some(registered.clone())
    );

    let (next, signed_next) = preparation(&workflow, Uuid::from_u128(18));
    let before = snapshot(&mut db);
    category(
        workflow.register(TOKEN, &next, &signed_next),
        OwnerCertificateError::ActiveBinding,
    );
    assert_eq!(snapshot(&mut db), before);
    clock.set(owner_binding_fixture::fixture().at + 2);
    let withdrawn = workflow.withdraw(TOKEN, first_id, 1).unwrap();
    assert_eq!(
        withdrawn.record.registration(),
        registered.record.registration()
    );
    assert_eq!(withdrawn.check, registered.check);
    assert_eq!(withdrawn.registered_at, registered.registered_at);
    assert_eq!(withdrawn.withdrawn_at, Some(clock.at()));
    clock.set(owner_binding_fixture::fixture().at + 3);
    let renewed = workflow.register(TOKEN, &next, &signed_next).unwrap();
    assert_eq!(
        renewed.record.registration().material().binding(),
        Uuid::from_u128(18)
    );
    assert_eq!(renewed.check.certificate, registered.check.certificate);
    assert_ne!(renewed.check.signature, registered.check.signature);
    assert_eq!(counts(&mut db), (2, 1, 3));
    assert_eq!(users(&mut db), original_users);
    assert_audit(
        &db,
        &[(&registered, false), (&withdrawn, true), (&renewed, false)],
    );
}

#[test]
fn historical_replay_ignores_new_account_counters_and_expiry_without_resurrecting_or_writing() {
    let Some((mut db, clock, trust)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let actor = principal(&mut db, owner);
    let repository = store(&db, &clock);
    let id = Uuid::from_u128(21);
    let first = capture::registration(repository.clone(), &clock, actor.clone(), id);
    let retry = capture::registration(repository.clone(), &clock, actor.clone(), id);
    let registered = applied(repository.commit_registration(first).unwrap());
    let workflow = service(repository.clone(), &clock, actor);
    clock.set(owner_binding_fixture::fixture().at + 1);
    let retired = workflow.withdraw(TOKEN, id, 1).unwrap();
    db.admin
        .execute(
            "UPDATE users SET revision=revision+1 WHERE id=$1",
            &[&owner.as_uuid()],
        )
        .unwrap();
    clock.set(trust.inspection.valid_until + 1);
    let before = snapshot(&mut db);
    assert_eq!(
        existing(repository.commit_registration(retry).unwrap()),
        retired
    );
    assert_eq!(workflow.withdraw(TOKEN, id, 1).unwrap(), retired);
    let reopened = store(&db, &clock);
    assert_eq!(reopened.find(owner, id).unwrap(), Some(retired.clone()));
    assert!(reopened.find(owner, Uuid::from_u128(22)).unwrap().is_none());
    category(
        reopened.load_withdrawal(owner, Uuid::from_u128(22)),
        OwnerCertificateError::NotFound,
    );
    let (fresh, signature) = preparation(&workflow, Uuid::from_u128(23));
    assert!(matches!(
        workflow.register(TOKEN, &fresh, &signature),
        Err(ApplicationError::OwnerCertificate(
            OwnerCertificateError::Credential(
                CredentialFailure::Expired | CredentialFailure::CrlExpired
            )
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
    assert_audit(&db, &[(&registered, false), (&retired, true)]);
}

#[test]
fn receipts_require_current_owner_and_retired_fingerprints_never_transfer_to_another_owner() {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    let owner = db.owner;
    let other = db.user("owner", false);
    let actor = principal(&mut db, owner);
    let other_actor = principal(&mut db, other);
    let repository = store(&db, &clock);
    let id = Uuid::from_u128(31);
    let transfer = capture::registration(
        repository.clone(),
        &clock,
        other_actor.clone(),
        Uuid::from_u128(32),
    );
    let occupied_uuid = capture::registration(repository.clone(), &clock, other_actor.clone(), id);
    let own = capture::registration(repository.clone(), &clock, actor.clone(), id);
    let registered = applied(repository.commit_registration(own).unwrap());
    let workflow = service(repository.clone(), &clock, actor);
    let retired = workflow.withdraw(TOKEN, id, 1).unwrap();
    let before = snapshot(&mut db);
    category(
        repository.commit_registration(transfer),
        OwnerCertificateError::FingerprintConflict,
    );
    category(
        repository.commit_registration(occupied_uuid),
        OwnerCertificateError::BindingConflict,
    );
    assert!(repository.find(other, id).unwrap().is_none());
    category(
        repository.load_withdrawal(other, id),
        OwnerCertificateError::NotFound,
    );
    assert_eq!(snapshot(&mut db), before);

    for role in ["litigator", "paralegal", "client"] {
        let denied = db.user(role, false);
        assert!(matches!(
            repository.load_registration(denied),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            repository.find(denied, id),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            repository.load_withdrawal(denied, id),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    let pending =
        capture::registration(repository.clone(), &clock, other_actor, Uuid::from_u128(33));
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
        &[&other.as_uuid()]).unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        repository.commit_registration(pending),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        repository.find(other, id),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(snapshot(&mut db), before);
    assert_audit(&db, &[(&registered, false), (&retired, true)]);
}
