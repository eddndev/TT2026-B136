//! Attempts to break the database guards of the sealed evidence manifest:
//! rows that differ from their receipt, a second or late manifest, a receipt
//! that loses its manifest, an export that would omit it and a runtime role
//! that tries to switch the guards off. Rationale: the section "Generation inside the acceptance
//! operation" of `docs/adr/0085-in-browser-evidence-validation.md`.
#![allow(dead_code)]
mod case_administration_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::{
    document_signing::{IndividualSigningStore, SigningCommit},
    documents::CaseDocumentStore,
};
use individual_document_backend_support::Harness;
use infrastructure::{PostgresCaseDocumentStore, PostgresIndividualSigningStore};
use postgres::error::SqlState;
use uuid::Uuid;

fn applied(test: &Harness, operation: Uuid) {
    let SigningCommit::Applied(_) = test.store().commit(test.command(operation)).unwrap() else {
        panic!("expected new individual signature")
    };
}

fn reopens(test: &Harness) -> bool {
    PostgresIndividualSigningStore::open(&test.db.runtime_url, test.clock.clone()).is_ok()
}

/// Runs `statements` as the schema owner with the manifest and cutoff
/// immutability triggers switched off, as only an owner could.
fn bypass_immutability(test: &mut Harness, statements: &str) {
    test.db
        .admin
        .batch_execute(&format!(
            "ALTER TABLE individual_document_manifests DISABLE TRIGGER individual_manifest_immutable;
            ALTER TABLE individual_manifest_cutoff DISABLE TRIGGER individual_manifest_cutoff_immutable;
            {statements};
            ALTER TABLE individual_document_manifests ENABLE TRIGGER individual_manifest_immutable;
            ALTER TABLE individual_manifest_cutoff ENABLE TRIGGER individual_manifest_cutoff_immutable"
        ))
        .unwrap();
}

#[test]
fn a_manifest_row_that_differs_from_its_receipt_commits_nothing() {
    for (trigger, change) in [
        (
            "aa_alter_manifest",
            "NEW.document_version:=NEW.document_version+1",
        ),
        (
            "aa_alter_manifest",
            "NEW.document_sha256:=sha256(NEW.document_sha256)",
        ),
        (
            "zz_alter_manifest",
            "NEW.document_sha256:=sha256(NEW.document_sha256)",
        ),
        (
            "zz_alter_manifest",
            "NEW.document_version:=NEW.document_version+1",
        ),
    ] {
        let Some(mut test) = Harness::new() else {
            return;
        };
        let command = test.command(Uuid::new_v4());
        let store = test.store();
        test.db
            .admin
            .batch_execute(&format!(
                "CREATE FUNCTION alter_test_manifest() RETURNS trigger LANGUAGE plpgsql
                    AS $$ BEGIN {change}; RETURN NEW; END $$;
                CREATE TRIGGER {trigger} BEFORE INSERT ON individual_document_manifests
                    FOR EACH ROW EXECUTE FUNCTION alter_test_manifest()"
            ))
            .unwrap();
        let before = test.snapshot();
        assert!(store.commit(command).is_err(), "{trigger}: {change}");
        assert_eq!(test.snapshot(), before, "{trigger}: {change}");
        assert_eq!(test.mutation_counts(), (0, 0), "{trigger}: {change}");
    }
}

#[test]
fn a_second_or_late_manifest_is_rejected_and_an_orphan_receipt_blocks_startup() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let operation = Uuid::new_v4();
    applied(&test, operation);
    let mut runtime = test.db.runtime();
    let copy = "INSERT INTO individual_document_manifests
        SELECT * FROM saved_test_manifest";
    runtime
        .batch_execute(
            "CREATE TEMPORARY TABLE saved_test_manifest AS
                SELECT * FROM individual_document_manifests",
        )
        .unwrap();
    let error = runtime.batch_execute(copy).unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::UNIQUE_VIOLATION));
    bypass_immutability(&mut test, "DELETE FROM individual_document_manifests");
    assert!(
        !reopens(&test),
        "a receipt after the cutoff lost its manifest"
    );

    bypass_immutability(
        &mut test,
        "UPDATE individual_manifest_cutoff SET first_audit_sequence=
            (SELECT max(audit_sequence)+1 FROM individual_document_signatures)",
    );
    assert!(
        reopens(&test),
        "a receipt before the cutoff keeps no manifest"
    );
    let error = runtime.batch_execute(copy).unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    assert!(reopens(&test));
}

#[test]
fn a_manifest_kept_for_a_receipt_before_the_cutoff_blocks_startup() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    applied(&test, Uuid::new_v4());
    assert!(reopens(&test));
    bypass_immutability(
        &mut test,
        "UPDATE individual_manifest_cutoff SET first_audit_sequence=
            (SELECT max(audit_sequence)+1 FROM individual_document_signatures)",
    );
    assert!(!reopens(&test));
}

#[test]
fn an_export_never_omits_the_manifest_a_receipt_after_the_cutoff_requires() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    applied(&test, Uuid::new_v4());
    let documents = PostgresCaseDocumentStore::open(&test.db.runtime_url).unwrap();
    assert!(documents.signing_manifest(&test.record).unwrap().is_some());
    bypass_immutability(&mut test, "DELETE FROM individual_document_manifests");
    assert!(
        documents.signing_manifest(&test.record).is_err(),
        "a receipt after the cutoff was exported without its manifest"
    );
    bypass_immutability(
        &mut test,
        "UPDATE individual_manifest_cutoff SET first_audit_sequence=
            (SELECT max(audit_sequence)+1 FROM individual_document_signatures)",
    );
    assert_eq!(documents.signing_manifest(&test.record).unwrap(), None);
}

#[test]
fn a_replay_or_read_never_returns_a_receipt_without_its_required_manifest() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let operation = Uuid::new_v4();
    applied(&test, operation);
    let store = test.store();
    let (actor, case, reference) = (test.actor.clone(), test.db.case, test.reference());
    let read =
        |store: &PostgresIndividualSigningStore| store.find(&actor, case, reference, operation);
    assert!(read(&store).unwrap().unwrap().manifest.is_some());
    bypass_immutability(&mut test, "DELETE FROM individual_document_manifests");
    assert!(
        read(&store).is_err(),
        "a receipt after the cutoff was read without its manifest"
    );
    bypass_immutability(
        &mut test,
        "UPDATE individual_manifest_cutoff SET first_audit_sequence=
            (SELECT max(audit_sequence)+1 FROM individual_document_signatures)",
    );
    assert_eq!(read(&store).unwrap().unwrap().manifest, None);
}

#[test]
fn the_runtime_role_cannot_switch_off_or_replace_the_manifest_guards() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    applied(&test, Uuid::new_v4());
    let mut runtime = test.db.runtime();
    for statement in [
        "ALTER TABLE individual_document_manifests DISABLE TRIGGER individual_manifest_immutable",
        "ALTER TABLE individual_document_signatures
            DISABLE TRIGGER individual_signature_manifest_pair",
        "DROP TRIGGER individual_manifest_register ON individual_document_manifests",
        "CREATE OR REPLACE FUNCTION individual_manifest_register() RETURNS trigger
            LANGUAGE plpgsql AS $$ BEGIN RETURN NEW; END $$",
        "SET session_replication_role=replica",
        "TRUNCATE individual_document_manifests",
        "DELETE FROM individual_document_manifests",
        "UPDATE individual_document_manifests SET manifest_token=manifest_token",
        "INSERT INTO individual_manifest_cutoff VALUES(TRUE,1)",
        "DELETE FROM individual_manifest_cutoff",
    ] {
        let error = runtime.batch_execute(statement).unwrap_err();
        assert_eq!(
            error.code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE),
            "{statement}"
        );
    }
    let error = test
        .db
        .admin
        .batch_execute("TRUNCATE individual_document_signatures CASCADE")
        .unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    assert!(reopens(&test));
}
