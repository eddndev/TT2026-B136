//! The sealed evidence manifest stored beside each individual signature
//! receipt, its database guards and its startup inventory. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.
#![allow(dead_code)]
mod case_administration_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::document_signing::{IndividualSigningStore, SigningCommit};
use individual_document_backend_support::Harness;
use infrastructure::PostgresIndividualSigningStore;
use postgres::error::SqlState;
use uuid::Uuid;

fn applied(test: &Harness, operation: Uuid) -> application::document_signing::SigningReceipt {
    let SigningCommit::Applied(receipt) = test.store().commit(test.command(operation)).unwrap()
    else {
        panic!("expected new individual signature")
    };
    receipt
}

fn reopens(test: &Harness) -> bool {
    PostgresIndividualSigningStore::open(&test.db.runtime_url, test.clock.clone()).is_ok()
}

#[test]
fn acceptance_stores_one_manifest_that_replay_and_reads_return_unchanged() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let operation = Uuid::new_v4();
    let duplicate = test.command(operation);
    let receipt = applied(&test, operation);
    let manifest = receipt.manifest.clone().expect("accepted with a manifest");
    let row = test
        .db
        .admin
        .query_one(
            "SELECT manifest,manifest_token,tsa_crl_pem,
                (SELECT count(*) FROM individual_document_manifests)
            FROM individual_document_manifests WHERE operation_id=$1",
            &[&operation],
        )
        .unwrap();
    assert_eq!(row.get::<_, Vec<u8>>(0), manifest.manifest);
    assert_eq!(row.get::<_, Vec<u8>>(1), manifest.token);
    assert_eq!(row.get::<_, Vec<u8>>(2), manifest.tsa_crl_pem);
    assert_eq!(row.get::<_, i64>(3), 1);
    assert_ne!(
        duplicate.manifest(),
        &manifest,
        "a resubmission obtains new tokens"
    );
    let SigningCommit::Existing(replayed) = test.store().commit(duplicate).unwrap() else {
        panic!("expected exact replay")
    };
    assert_eq!(replayed, receipt);
    assert_eq!(
        test.store()
            .find(&test.actor, test.db.case, test.reference(), operation)
            .unwrap(),
        Some(receipt)
    );
    test.db.migrate();
    assert!(reopens(&test));
}

#[test]
fn a_receipt_whose_manifest_insert_is_skipped_fails_at_commit() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let command = test.command(Uuid::new_v4());
    let store = test.store();
    test.db
        .admin
        .batch_execute(
            "CREATE FUNCTION skip_test_manifest() RETURNS trigger LANGUAGE plpgsql
                AS $$ BEGIN RETURN NULL; END $$;
            CREATE TRIGGER aa_skip_test_manifest BEFORE INSERT ON individual_document_manifests
                FOR EACH ROW EXECUTE FUNCTION skip_test_manifest()",
        )
        .unwrap();
    let before = test.snapshot();
    assert!(store.commit(command).is_err());
    assert_eq!(test.snapshot(), before);
    assert_eq!(test.mutation_counts(), (0, 0));
}

#[test]
fn stored_manifests_and_the_cutoff_reject_update_delete_and_truncate() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    applied(&test, Uuid::new_v4());
    let mut runtime = test.db.runtime();
    for statement in [
        "UPDATE individual_document_manifests SET manifest_token=manifest_token",
        "DELETE FROM individual_document_manifests",
        "UPDATE individual_manifest_cutoff SET first_audit_sequence=first_audit_sequence+1",
        "INSERT INTO individual_manifest_cutoff VALUES(TRUE,1)",
    ] {
        assert!(runtime.batch_execute(statement).is_err(), "{statement}");
    }
    for statement in [
        "UPDATE individual_document_manifests SET manifest_token=manifest_token",
        "DELETE FROM individual_document_manifests",
        "TRUNCATE individual_document_manifests",
        "UPDATE individual_manifest_cutoff SET first_audit_sequence=first_audit_sequence+1",
        "DELETE FROM individual_manifest_cutoff",
    ] {
        let error = test.db.admin.batch_execute(statement).unwrap_err();
        assert_eq!(
            error.code(),
            Some(&SqlState::CHECK_VIOLATION),
            "{statement}"
        );
    }
}

#[test]
fn the_receipt_table_counts_the_manifest_triggers() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let count: i64 = test
        .db
        .admin
        .query_one(
            "SELECT count(*) FROM pg_trigger
            WHERE tgrelid='individual_document_signatures'::regclass",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 17);
    test.db
        .admin
        .batch_execute(
            "ALTER TABLE individual_document_signatures
                DISABLE TRIGGER individual_signature_manifest_pair",
        )
        .unwrap();
    assert!(!reopens(&test));
    test.db
        .admin
        .batch_execute(
            "ALTER TABLE individual_document_signatures
                ENABLE TRIGGER individual_signature_manifest_pair",
        )
        .unwrap();
    assert!(reopens(&test));
}

#[test]
fn startup_rejects_a_tampered_manifest_token_or_crl() {
    for (column, replacement) in [
        ("manifest_token", "tsa_crl_pem"),
        ("tsa_crl_pem", "manifest_token"),
    ] {
        let Some(mut test) = Harness::new() else {
            return;
        };
        applied(&test, Uuid::new_v4());
        assert!(reopens(&test));
        test.db
            .admin
            .batch_execute(&format!(
                "ALTER TABLE individual_document_manifests DISABLE TRIGGER individual_manifest_immutable;
                UPDATE individual_document_manifests SET {column}={replacement};
                ALTER TABLE individual_document_manifests ENABLE TRIGGER individual_manifest_immutable"
            ))
            .unwrap();
        assert!(!reopens(&test), "{column}");
    }
}
