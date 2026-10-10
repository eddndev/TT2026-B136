mod case_administration_support;
#[allow(dead_code)]
mod declaration_fixture;
#[allow(dead_code)]
mod individual_signing_fixture;
#[path = "signing_binding_support/mod.rs"]
mod support;
use application::ApplicationError;
use domain::clock::Clock as _;
use support::*;
use uuid::Uuid;

#[test]
fn binding_is_account_scoped_and_rejects_other_roles_without_case_membership() {
    let Some((mut db, clock, actor)) = fixture() else {
        return;
    };
    let store = store(&db, &clock);
    let service = service(store.clone(), &clock, actor.clone());
    let receipt = register(&service, Uuid::new_v4());
    assert_eq!(store.find_current(actor.id).unwrap(), Some(receipt.clone()));
    let other = db.user("litigator", false);
    assert_eq!(
        store
            .find(other, receipt.statement.material().binding())
            .unwrap(),
        None
    );
    for role in ["owner", "paralegal", "client"] {
        let user = db.user(role, false);
        assert!(matches!(
            store.load_context(user),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            store.find_current(user),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&actor.id.as_uuid()]).unwrap();
    assert!(matches!(
        store.find_current(actor.id),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(counts(&mut db), (1, 0, 1));
}

#[test]
fn exact_replay_and_terminal_withdrawal_preserve_the_original_evidence_after_expiry() {
    let Some((mut db, clock, actor)) = fixture() else {
        return;
    };
    let store = store(&db, &clock);
    let id = Uuid::new_v4();
    let first = command(store.clone(), &clock, actor.clone(), id);
    let replay = command(store.clone(), &clock, actor.clone(), id);
    let receipt = store.commit_binding(first).unwrap();
    clock.set(receipt.check.valid_until + 100);
    assert_eq!(store.commit_binding(replay).unwrap(), receipt);
    let context = store.load_context(actor.id).unwrap();
    let retired = store
        .withdraw(&actor, id, context.account, clock.now())
        .unwrap();
    assert_eq!(
        store
            .withdraw(&actor, id, context.account, clock.now())
            .unwrap(),
        retired
    );
    assert_eq!(retired.check, receipt.check);
    assert_eq!(store.find_current(actor.id).unwrap(), None);
    assert_eq!(counts(&mut db), (1, 1, 2));
}

#[test]
fn fingerprint_ownership_is_permanent_and_one_account_cannot_hold_two_live_bindings() {
    let Some((mut db, clock, actor)) = fixture() else {
        return;
    };
    let store = store(&db, &clock);
    let first = command(store.clone(), &clock, actor.clone(), Uuid::new_v4());
    let second = command(store.clone(), &clock, actor.clone(), Uuid::new_v4());
    let other = db.user("litigator", false);
    let other_actor = principal(&mut db, other);
    let foreign = command(store.clone(), &clock, other_actor, Uuid::new_v4());
    let receipt = store.commit_binding(first).unwrap();
    category(store.commit_binding(second), SigningWorkflowError::Conflict);
    let context = store.load_context(actor.id).unwrap();
    store
        .withdraw(
            &actor,
            receipt.statement.material().binding(),
            context.account,
            clock.now(),
        )
        .unwrap();
    category(
        store.commit_binding(foreign),
        SigningWorkflowError::Conflict,
    );
    assert_eq!(counts(&mut db), (1, 1, 2));
}

#[test]
fn concurrent_exact_registration_commits_one_receipt_and_one_mutation_audit() {
    let Some((mut db, clock, actor)) = fixture() else {
        return;
    };
    let left = store(&db, &clock);
    let right = store(&db, &clock);
    let id = Uuid::new_v4();
    let first = command(left.clone(), &clock, actor.clone(), id);
    let second = command(right.clone(), &clock, actor, id);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other = barrier.clone();
    let thread = std::thread::spawn(move || {
        other.wait();
        left.commit_binding(first).unwrap()
    });
    barrier.wait();
    let receipt = right.commit_binding(second).unwrap();
    assert_eq!(thread.join().unwrap(), receipt);
    assert_eq!(counts(&mut db), (1, 0, 1));
}

#[test]
fn account_and_trust_changes_after_preparation_reject_and_audit_failure_rolls_back() {
    let Some((mut db, clock, actor)) = fixture() else {
        return;
    };
    let store = store(&db, &clock);
    let changed = command(store.clone(), &clock, actor.clone(), Uuid::new_v4());
    db.admin
        .execute(
            "UPDATE users SET revision=revision+1 WHERE id=$1",
            &[&actor.id.as_uuid()],
        )
        .unwrap();
    category(
        store.commit_binding(changed),
        SigningWorkflowError::AccountChanged,
    );
    let changed_trust = command(store.clone(), &clock, actor.clone(), Uuid::new_v4());
    publish_successor(&db, &clock, changed_trust.prepared().trust());
    category(
        store.commit_binding(changed_trust),
        SigningWorkflowError::TrustChanged,
    );
    let expired = command(store.clone(), &clock, actor.clone(), Uuid::new_v4());
    let at = clock.now();
    clock.set(expired.check().valid_until + 1);
    category(
        store.commit_binding(expired),
        SigningWorkflowError::Credential(domain::crypto::CredentialFailure::Expired),
    );
    clock.set(at.unix_timestamp());
    let command = command(store.clone(), &clock, actor, Uuid::new_v4());
    let before = snapshot(&mut db);
    db.admin.batch_execute("CREATE FUNCTION reject_binding_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.action='document.signer_bound' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$;
        CREATE TRIGGER reject_binding_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_binding_audit()").unwrap();
    assert!(store.commit_binding(command).is_err());
    assert_eq!(snapshot(&mut db), before);
}

fn publish_successor(
    db: &Fixture,
    clock: &std::sync::Arc<Clock>,
    prior: &application::credential_trust::CredentialTrustSnapshot,
) {
    use application::credential_trust::{CredentialTrustExpectation, CredentialTrustStore};
    use der::{
        asn1::{OctetString, Uint},
        oid::AssociatedOid,
        Decode, Encode,
    };
    use domain::crypto::InternalDeclarationVerifier;
    use x509_cert::{crl::CertificateList, ext::pkix::CrlNumber};
    let mut crl = CertificateList::from_der(&prior.inspection.crl_der).unwrap();
    let number = prior.inspection.crl_number + 1;
    let ext = crl
        .tbs_cert_list
        .crl_extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|ext| ext.extn_id == CrlNumber::OID)
        .unwrap();
    ext.extn_value = OctetString::new(
        CrlNumber(Uint::new(&number.to_be_bytes()).unwrap())
            .to_der()
            .unwrap(),
    )
    .unwrap();
    let inspection = infrastructure::certificates::InternalRsaDeclarationVerifier::new()
        .inspect_trust(
            &prior.inspection.root_der,
            &declaration_fixture::signed_crl(crl),
            clock.now().unix_timestamp(),
        )
        .unwrap();
    infrastructure::PostgresCredentialTrustStore::open(&db.admin_url, clock.clone())
        .unwrap()
        .publish(
            CredentialTrustExpectation::Revision(prior.revision),
            inspection,
        )
        .unwrap();
}

#[test]
fn runtime_cannot_modify_history_and_startup_rejects_a_removed_payload_guard() {
    let Some((mut db, clock, actor)) = fixture() else {
        return;
    };
    let service = service(store(&db, &clock), &clock, actor);
    register(&service, Uuid::new_v4());
    let mut runtime = db.runtime();
    for sql in [
        "UPDATE document_signing_bindings SET signature=signature",
        "DELETE FROM document_signing_bindings",
        "TRUNCATE document_signing_bindings CASCADE",
        "ALTER TABLE document_signing_bindings DISABLE TRIGGER ALL",
    ] {
        assert!(runtime.batch_execute(sql).is_err());
    }
    let constraint: String = db
        .admin
        .query_one(
            "SELECT quote_ident(conname) FROM pg_constraint
        WHERE conrelid='document_signing_bindings'::regclass AND contype='c'
            AND pg_get_expr(conbin,conrelid)='(statement_digest = sha256(statement))'",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin
        .batch_execute(&format!(
            "ALTER TABLE document_signing_bindings DROP CONSTRAINT {constraint}"
        ))
        .unwrap();
    assert!(PostgresSigningBindingStore::open(&db.runtime_url, clock).is_err());
}

#[test]
fn migration_restore_and_inventory_preserve_history_and_fail_closed_on_tampering() {
    let Some((mut db, clock, actor)) = fixture() else {
        return;
    };
    let service = service(store(&db, &clock), &clock, actor.clone());
    let receipt = register(&service, Uuid::new_v4());
    drop(service);
    db.migrate();
    let before = snapshot(&mut db);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("signing.dump");
    let output = std::process::Command::new("pg_dump")
        .args([
            "--dbname",
            &db.admin_url,
            "--schema",
            &db.schema,
            "--format=custom",
            "--file",
        ])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    let output = std::process::Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &db.admin_url])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        store(&db, &clock)
            .find(actor.id, receipt.statement.material().binding())
            .unwrap(),
        Some(receipt)
    );
    db.admin
        .batch_execute(
            "ALTER TABLE document_signing_bindings DISABLE TRIGGER document_signing_immutable;
        UPDATE document_signing_bindings SET signature=decode(repeat('00',384),'hex');
        ALTER TABLE document_signing_bindings ENABLE TRIGGER document_signing_immutable",
        )
        .unwrap();
    assert!(PostgresSigningBindingStore::open(&db.runtime_url, clock).is_err());
}
