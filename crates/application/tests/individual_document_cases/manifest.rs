use crate::{document_signing_support::failure, individual_document_support::*};
use application::ApplicationError;
use domain::{crypto::DocumentHasher, DomainError};
use evidence_verify::manifest::{self, role_entry, Role};

fn submit(
    harness: Harness,
) -> Result<application::document_signing::SigningReceipt, ApplicationError> {
    harness.service().submit(
        "session",
        &consent(),
        &pdf_signature(),
        &consent_signature(),
    )
}

#[test]
fn acceptance_time_stamps_a_manifest_of_the_final_sealed_entries() {
    let mut harness = Harness::new();
    harness.verify_with(|_, _| {});
    let state = harness.state.clone();
    let receipt = submit(harness).unwrap();
    let stored = receipt.manifest.clone().unwrap();
    assert_eq!(stored.token, MANIFEST_TOKEN);
    assert_eq!(stored.tsa_crl_pem, application_pem(b"crl"));
    let state = state.lock().unwrap();
    assert_eq!((state.tsa_calls, state.commits), (2, 1));
    assert_eq!(state.manifest_digest, Some(stored.digest()));
    let parsed = manifest::parse(&stored.manifest).unwrap();
    assert_eq!(parsed.operation_id, operation().to_string());
    assert_eq!(parsed.case_id, case().as_uuid().to_string());
    assert_eq!(parsed.artifacts.len(), Role::ALL.len());
    let evidence = &receipt.evidence;
    let individual = evidence.individual.as_ref().unwrap();
    for (role, bytes) in [
        (Role::DocumentSignature, evidence.signature.as_slice()),
        (Role::DocumentTimestamp, evidence.timestamp_token.as_slice()),
        (Role::TsaChain, ROOT_PEM),
        (Role::TsaIssuerCrl, stored.tsa_crl_pem.as_slice()),
        (
            Role::ConsentStatement,
            individual.consent_statement.as_slice(),
        ),
        (Role::BindingCrl, individual.binding_crl_der.as_slice()),
    ] {
        let row = parsed
            .artifacts
            .iter()
            .find(|row| row.role == role)
            .unwrap();
        assert_eq!(row.entry, role_entry(role, "selected.pdf"));
        assert_eq!(row.sha256, evidence_verify::certificate::sha256(bytes));
        assert_eq!(row.size, bytes.len() as u64);
    }
    let document = parsed
        .artifacts
        .iter()
        .find(|row| row.role == Role::Document)
        .unwrap();
    assert_eq!(
        (document.sha256, document.size),
        (
            *document_signing_support_hash().as_bytes(),
            PDF.len() as u64
        )
    );
    assert_eq!(parsed.signers[0].binding_registered_at, 1000);
}

fn document_signing_support_hash() -> domain::crypto::Sha256Digest {
    crate::document_signing_support::Hasher.hash_bytes(PDF)
}

#[test]
fn a_failed_or_unordered_manifest_token_or_crl_commits_nothing() {
    for case in 0..3 {
        let mut harness = Harness::new();
        harness.verify_with(|_, _| {});
        let state = harness.state.clone();
        {
            let mut value = state.lock().unwrap();
            match case {
                0 => value.manifest_failure = Some(ManifestFailure::Rejected),
                1 => value.manifest_failure = Some(ManifestFailure::Configuration),
                _ => value.manifest_gen_time = value.document_gen_time - 1,
            }
        }
        let error = failure(submit(harness));
        match case {
            1 => assert!(matches!(error, ApplicationError::InvalidConfiguration(_))),
            _ => assert!(matches!(
                error,
                ApplicationError::TimestampEvidenceRejected(_)
            )),
        }
        let value = state.lock().unwrap();
        assert_eq!((value.tsa_calls, value.commits), (2, 0));
    }
}

#[test]
fn a_manifest_token_issued_after_acceptance_commits_nothing() {
    let mut harness = Harness::new();
    harness.verify_with(|_, _| {});
    let state = harness.state.clone();
    state.lock().unwrap().manifest_gen_time = 1002;
    assert!(matches!(
        failure(submit(harness)),
        ApplicationError::TimestampEvidenceRejected(_)
    ));
    let value = state.lock().unwrap();
    assert_eq!((value.tsa_calls, value.commits), (2, 0));
}

#[test]
fn an_anchor_other_than_the_trust_root_requests_no_token() {
    for anchor in [
        b"-----BEGIN CERTIFICATE-----\nb3RoZXI=\n-----END CERTIFICATE-----\n".as_slice(),
        b"not a certificate".as_slice(),
    ] {
        let mut harness = Harness::with_anchor(anchor);
        harness.verify_with(|_, _| {});
        let state = harness.state.clone();
        assert!(matches!(
            failure(submit(harness)),
            ApplicationError::InvalidConfiguration(_)
        ));
        let value = state.lock().unwrap();
        assert_eq!((value.tsa_calls, value.commits), (0, 0));
    }
}

#[test]
fn a_reserved_document_name_is_rejected_before_any_token() {
    for name in [
        "Manifiesto.JSON",
        "tsa-crl.pem",
        "RECIBO-ACEPTACION.json",
        "manifiesto.json.tsr",
        "Document-Consent",
        "ca.pem",
    ] {
        let harness = Harness::new();
        let state = harness.state.clone();
        state.lock().unwrap().record.name = name.into();
        assert!(matches!(
            failure(harness.service().prepare(
                "session",
                operation(),
                case(),
                reference(),
                &pdf_signature()
            )),
            ApplicationError::Domain(DomainError::InvalidArchiveEntryName { .. })
        ));
        let value = state.lock().unwrap();
        assert_eq!(
            (value.format_calls, value.tsa_calls, value.commits),
            (0, 0, 0)
        );
    }
}

#[test]
fn a_stored_manifest_that_names_another_operation_is_inconsistent() {
    let mut harness = Harness::new();
    harness.verify_with(|_, _| {});
    let state = harness.state.clone();
    let service = harness.service();
    service
        .submit(
            "session",
            &consent(),
            &pdf_signature(),
            &consent_signature(),
        )
        .unwrap();
    state
        .lock()
        .unwrap()
        .found
        .as_mut()
        .unwrap()
        .manifest
        .as_mut()
        .unwrap()
        .manifest
        .push(b' ');
    assert!(matches!(
        failure(service.get("session", case(), reference(), operation())),
        ApplicationError::StoredDocumentInconsistent(_)
    ));
}
