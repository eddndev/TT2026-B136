#![allow(dead_code)]
mod case_administration_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::document_signing::{IndividualSigningStore, SigningBindingStore, SigningCommit};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use domain::clock::Clock as _;
use individual_document_backend_support::Harness;
use infrastructure::{documents::encode_document_record, PostgresIndividualSigningStore};
use postgres::error::SqlState;
use uuid::Uuid;

#[test]
fn personal_document_evidence_without_a_receipt_fails_at_commit() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let command = test.command(Uuid::new_v4());
    let record: serde_json::Value =
        serde_json::from_slice(&encode_document_record(command.record()).unwrap()).unwrap();
    let evidence = &record["evidence"];
    assert!(evidence["individual"].is_object());
    let before = test.snapshot();
    let mut runtime = test.db.runtime();
    let mut tx = runtime.transaction().unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE documents SET evidence=$1 WHERE id=$2 AND version=$3",
            &[
                evidence,
                &test.record.id.as_uuid(),
                &i64::from(test.record.version.get())
            ]
        )
        .unwrap(),
        1
    );
    let error = tx.commit().unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    assert_eq!(test.snapshot(), before);
    assert_eq!(test.mutation_counts(), (0, 0));
}

#[test]
fn divergent_receipt_or_evidence_rolls_back_the_document_and_audit() {
    for mutation in [DIVERGENT_RECEIPT, DIVERGENT_EVIDENCE] {
        let Some(mut test) = Harness::new() else {
            return;
        };
        let command = test.command(Uuid::new_v4());
        let store = test.store();
        test.db.admin.batch_execute(mutation).unwrap();
        let before = test.snapshot();
        assert!(store.commit(command).is_err());
        assert_eq!(test.snapshot(), before);
        assert_eq!(test.mutation_counts(), (0, 0));
    }
}

const DIVERGENT_RECEIPT: &str = "CREATE FUNCTION alter_test_receipt() RETURNS trigger
    LANGUAGE plpgsql AS $$ BEGIN
        NEW.consent_signature:=set_byte(NEW.consent_signature,0,get_byte(NEW.consent_signature,0) # 1);
        RETURN NEW;
    END $$;
    CREATE TRIGGER zz_alter_test_receipt BEFORE INSERT ON individual_document_signatures
        FOR EACH ROW EXECUTE FUNCTION alter_test_receipt()";

const DIVERGENT_EVIDENCE: &str = "CREATE FUNCTION alter_test_evidence() RETURNS trigger
    LANGUAGE plpgsql AS $$ BEGIN
        NEW.evidence:=jsonb_set(NEW.evidence,'{individual,consent_signature,0}',
            to_jsonb((NEW.evidence#>>'{individual,consent_signature,0}')::integer # 1));
        RETURN NEW;
    END $$;
    CREATE TRIGGER zz_alter_test_evidence BEFORE UPDATE ON documents
        FOR EACH ROW EXECUTE FUNCTION alter_test_evidence()";

#[test]
fn disabling_either_deferred_pair_guard_prevents_reopening() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    for (table, trigger) in [
        ("documents", "individual_signature_document_pair"),
        (
            "individual_document_signatures",
            "individual_signature_receipt_pair",
        ),
    ] {
        test.db
            .admin
            .batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER {trigger}"))
            .unwrap();
        assert!(
            PostgresIndividualSigningStore::open(&test.db.runtime_url, test.clock.clone()).is_err()
        );
        test.db
            .admin
            .batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER {trigger}"))
            .unwrap();
        assert!(
            PostgresIndividualSigningStore::open(&test.db.runtime_url, test.clock.clone()).is_ok()
        );
    }
}

#[test]
fn startup_rejects_a_valid_successor_crl_substituted_for_acceptance_trust() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let registration = signing_binding_support::store(&test.db, &test.clock)
        .load_context(test.actor.id)
        .unwrap()
        .trust
        .unwrap();
    let acceptance = publish_successor(&test, &registration);
    let command = test.command(Uuid::new_v4());
    let SigningCommit::Applied(receipt) = test.store().commit(command).unwrap() else {
        panic!("expected new individual signature")
    };
    assert_eq!(registration.revision.get(), 1);
    assert_eq!(receipt.consent.material().trust_revision(), 2);
    let successor = publish_successor(&test, &acceptance);
    assert_eq!(successor.revision.get(), 3);
    assert!(PostgresIndividualSigningStore::open(&test.db.runtime_url, test.clock.clone()).is_ok());

    let mut evidence = receipt.evidence.clone();
    evidence.crl_pem = crl_pem(&successor.inspection.crl_der);
    evidence
        .individual
        .as_ref()
        .unwrap()
        .validate(
            &test.record,
            &evidence,
            &infrastructure::RingSha256Hasher,
            &infrastructure::certificates::InternalRsaDeclarationVerifier,
        )
        .unwrap();
    let before = test.snapshot();
    let mut tx = test.db.admin.transaction().unwrap();
    tx.batch_execute(
        "ALTER TABLE documents DISABLE TRIGGER documents_preserve_evidence;
        ALTER TABLE documents DISABLE TRIGGER individual_signature_document_pair",
    )
    .unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE documents SET evidence=jsonb_set(evidence,'{crl_base64}',to_jsonb($1::text))
            WHERE id=$2 AND version=$3",
            &[
                &STANDARD.encode(&evidence.crl_pem),
                &test.record.id.as_uuid(),
                &i64::from(test.record.version.get()),
            ],
        )
        .unwrap(),
        1
    );
    tx.batch_execute(
        "ALTER TABLE documents ENABLE TRIGGER individual_signature_document_pair;
        ALTER TABLE documents ENABLE TRIGGER documents_preserve_evidence",
    )
    .unwrap();
    tx.commit().unwrap();
    let after = test.snapshot();
    assert_eq!(after["receipts"], before["receipts"]);
    assert_eq!(after["audit"], before["audit"]);
    assert_ne!(after["documents"], before["documents"]);
    assert!(
        PostgresIndividualSigningStore::open(&test.db.runtime_url, test.clock.clone()).is_err(),
        "startup must match captured acceptance CRL to the receipt's exact trust revision"
    );
}

fn publish_successor(
    test: &Harness,
    prior: &application::credential_trust::CredentialTrustSnapshot,
) -> application::credential_trust::CredentialTrustSnapshot {
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
            test.clock.now().unix_timestamp(),
        )
        .unwrap();
    infrastructure::PostgresCredentialTrustStore::open(&test.db.admin_url, test.clock.clone())
        .unwrap()
        .publish(
            CredentialTrustExpectation::Revision(prior.revision),
            inspection,
        )
        .unwrap()
}

fn crl_pem(der: &[u8]) -> Vec<u8> {
    let encoded = STANDARD.encode(der);
    let mut pem = "-----BEGIN X509 CRL-----\n".to_owned();
    for line in encoded.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(line).unwrap());
        pem.push('\n');
    }
    pem.push_str("-----END X509 CRL-----\n");
    pem.into_bytes()
}

#[test]
fn logical_restore_preserves_signed_versions_after_later_binding_withdrawal() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let operation = Uuid::new_v4();
    let command = test.command(operation);
    let SigningCommit::Applied(receipt) = test.store().commit(command).unwrap() else {
        panic!("expected new individual signature")
    };
    let binding = signing_binding_support::store(&test.db, &test.clock);
    let context = binding.load_context(test.actor.id).unwrap();
    binding
        .withdraw(
            &test.actor,
            test.binding_id,
            context.account,
            test.clock.now(),
        )
        .unwrap();
    drop(binding);
    test.db.migrate();
    let before = test.snapshot();
    let binding_before = signing_binding_support::snapshot(&mut test.db);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("individual-signature.dump");
    let output = std::process::Command::new("pg_dump")
        .args([
            "--dbname",
            &test.db.admin_url,
            "--schema",
            &test.db.schema,
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
    test.db
        .control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", test.db.schema))
        .unwrap();
    let output = std::process::Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &test.db.admin_url])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(test.snapshot(), before);
    assert_eq!(
        signing_binding_support::snapshot(&mut test.db),
        binding_before
    );
    test.db.migrate();
    assert_eq!(test.snapshot(), before);
    let store = test.store();
    assert_eq!(
        store
            .find(&test.actor, test.db.case, test.reference(), operation)
            .unwrap(),
        Some(receipt)
    );
    let state = store
        .verification_context(&test.actor, test.db.case, test.reference())
        .unwrap();
    assert_eq!(state.binding_withdrawn_at, Some(test.clock.now()));
    assert_eq!(test.mutation_counts(), (1, 1));
}
