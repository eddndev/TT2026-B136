//! Who may hold a document-signing binding, and what it never authorizes:
//! docs/adr/0086-bilateral-contract-signing.md.
mod case_administration_support;
#[allow(dead_code)]
mod declaration_fixture;
#[allow(dead_code)]
mod individual_signing_fixture;
#[allow(dead_code)]
#[path = "signing_binding_support/mod.rs"]
mod support;
use domain::{clock::Clock as _, identity::UserId};
use support::*;
use uuid::Uuid;

#[test]
fn owner_and_client_accounts_register_and_withdraw_only_their_own_binding() {
    for role in ["owner", "client"] {
        let Some((mut db, clock, litigator)) = fixture() else {
            return;
        };
        let user = db.user(role, false);
        let party = principal(&mut db, user);
        let store = store(&db, &clock);
        let receipt = register(
            &service(store.clone(), &clock, party.clone()),
            Uuid::new_v4(),
        );
        let binding = receipt.statement.material().binding();
        assert_eq!(receipt.statement.account().id(), user);
        assert_eq!(store.find_current(user).unwrap(), Some(receipt.clone()));
        assert_eq!(store.find(litigator.id, binding).unwrap(), None);
        let context = store.load_context(user).unwrap();
        let retired = store
            .withdraw(&party, binding, context.account, clock.now())
            .unwrap();
        assert_eq!(retired.check, receipt.check);
        assert_eq!(store.find_current(user).unwrap(), None);
        assert_eq!(counts(&mut db), (1, 1, 2));
    }
}

#[test]
fn the_storage_guard_still_rejects_a_paralegal_binding() {
    let Some((mut db, _clock, _)) = fixture() else {
        return;
    };
    let paralegal = db.user("paralegal", false);
    let error = db
        .admin
        .execute(
            "INSERT INTO document_signing_bindings(binding_id,signer_id,actor_email,
            account_revision,auth_generation) SELECT $1,id,email,revision,auth_generation
            FROM users WHERE id=$2",
            &[&Uuid::new_v4(), &paralegal.as_uuid()],
        )
        .unwrap_err();
    assert_eq!(
        error.as_db_error().map(|e| (e.code().code(), e.message())),
        Some((
            "42501",
            "document signing actor is not currently authorized"
        ))
    );
}

#[test]
fn startup_rejects_the_superseded_litigator_only_binding_guard() {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    db.admin
        .batch_execute(include_str!(
            "../../../migrations/0044_document_signing_bindings.sql"
        ))
        .unwrap();
    assert!(PostgresSigningBindingStore::open(&db.runtime_url, clock.clone()).is_err());
    db.migrate();
    assert!(PostgresSigningBindingStore::open(&db.runtime_url, clock).is_ok());
}

/// The receipt guard of individual signatures checks the signer role before
/// any other column, so an incomplete row isolates that check.
fn individual_signature_error(db: &mut Fixture, signer: UserId, binding: Uuid) -> (String, String) {
    let error = db
        .admin
        .execute(
            "INSERT INTO individual_document_signatures(operation_id,case_id,signer_id,
            binding_id,actor_email,account_revision,auth_generation)
            SELECT $1,$2,id,$3,email,revision,auth_generation FROM users WHERE id=$4",
            &[
                &Uuid::new_v4(),
                &db.case.as_uuid(),
                &binding,
                &signer.as_uuid(),
            ],
        )
        .unwrap_err();
    let error = error.as_db_error().unwrap();
    (error.code().code().to_owned(), error.message().to_owned())
}

#[test]
fn an_owner_or_client_binding_never_produces_an_individual_signature() {
    let rejected = (
        "42501".to_owned(),
        "individual signer is not currently authorized".to_owned(),
    );
    for role in ["owner", "client"] {
        let Some((mut db, clock, litigator)) = fixture() else {
            return;
        };
        let user = db.user(role, role == "owner");
        let party = principal(&mut db, user);
        let receipt = register(&service(store(&db, &clock), &clock, party), Uuid::new_v4());
        let binding = receipt.statement.material().binding();
        assert_eq!(individual_signature_error(&mut db, user, binding), rejected);
        db.admin
            .execute(
                "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
                &[&db.case.as_uuid(), &litigator.id.as_uuid()],
            )
            .unwrap();
        assert_ne!(
            individual_signature_error(&mut db, litigator.id, binding),
            rejected
        );
        let signatures: i64 = db
            .admin
            .query_one("SELECT count(*) FROM individual_document_signatures", &[])
            .unwrap()
            .get(0);
        assert_eq!(signatures, 0);
    }
}
