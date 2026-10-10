#[allow(dead_code)]
mod case_document_support;
#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod document_signing_support;
mod individual_document_support;
#[allow(dead_code)]
#[path = "support/individual_evidence.rs"]
mod proofs;
#[path = "individual_document_cases/rejections.rs"]
mod rejections;
#[path = "individual_document_cases/verification.rs"]
mod verification;

use application::{document_signing::SigningWorkflowError as Error, ApplicationError};
use document_signing_support::{failure, kind};
use individual_document_support::*;

#[test]
fn preparation_revalidates_the_exact_pdf_and_captures_its_signing_identity() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let prepared = harness
        .service()
        .prepare(
            "session",
            operation(),
            case(),
            reference(),
            &pdf_signature(),
        )
        .unwrap();
    assert_eq!(prepared.consent(), &consent());
    assert_eq!(prepared.name(), "selected.pdf");
    assert_eq!(
        prepared.certificate_fingerprint(),
        document_signing_support::certificate().fingerprint
    );
    let state = state.lock().unwrap();
    assert_eq!(
        (state.format_calls, state.tsa_calls, state.commits),
        (1, 0, 0)
    );
}

#[test]
fn external_document_and_consent_proofs_commit_exact_public_evidence_once() {
    let mut harness = Harness::new();
    harness.verify_with(|_, _| {});
    let state = harness.state.clone();
    let result = harness
        .service()
        .submit(
            "session",
            &consent(),
            &pdf_signature(),
            &consent_signature(),
        )
        .unwrap();
    assert_eq!(result.consent, consent());
    assert_eq!(result.evidence.signature, pdf_signature().as_bytes());
    let individual = result.evidence.individual.unwrap();
    assert_eq!(
        individual.binding_statement,
        document_signing_support::statement().canonical_bytes()
    );
    assert_eq!(individual.consent_statement, consent().canonical_bytes());
    assert_eq!(individual.binding_registered_at, 1000);
    assert_eq!(individual.accepted_at, 1000);
    let state = state.lock().unwrap();
    assert_eq!((state.tsa_calls, state.commits), (1, 1));
    assert!(state.record.evidence.is_none());
}

#[test]
fn exact_replay_and_receipt_read_survive_retirement_and_expiry_without_new_crypto() {
    let mut harness = Harness::new();
    harness.verify_with(|_, _| {});
    let state = harness.state.clone();
    let authority = harness.binding.state.clone();
    let service = harness.service();
    let original = service
        .submit(
            "session",
            &consent(),
            &pdf_signature(),
            &consent_signature(),
        )
        .unwrap();
    {
        let mut value = authority.lock().unwrap();
        value.now = 5000;
        value.context.trust = None;
        value.found = None;
    }
    state.lock().unwrap().deny_document = true;
    assert_eq!(
        service
            .submit(
                "session",
                &consent(),
                &pdf_signature(),
                &consent_signature()
            )
            .unwrap(),
        original
    );
    assert_eq!(
        service
            .get("session", case(), reference(), operation())
            .unwrap(),
        original
    );
    let state = state.lock().unwrap();
    assert_eq!((state.tsa_calls, state.commits), (1, 1));
}

#[test]
fn replay_with_another_signature_conflicts_and_changed_principal_cannot_read_a_receipt() {
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
    let other = domain::crypto::Signature::from_bytes(vec![9; 384]).unwrap();
    for (pdf, proof) in [(&other, &consent_signature()), (&pdf_signature(), &other)] {
        assert_eq!(
            kind(failure(service.submit("session", &consent(), pdf, proof))),
            Error::Conflict
        );
    }
    state.lock().unwrap().change_on_find = true;
    assert!(matches!(
        failure(service.get("session", case(), reference(), operation())),
        ApplicationError::InvalidSession
    ));
}

#[test]
fn concurrent_exact_commit_returns_first_evidence_despite_another_timestamp() {
    let mut first = Harness::new();
    first.verify_with(|_, _| {});
    let mut original = first
        .service()
        .submit(
            "session",
            &consent(),
            &pdf_signature(),
            &consent_signature(),
        )
        .unwrap();
    original.evidence.openssl_version = "original captured timestamp implementation".into();
    let mut contender = Harness::new();
    contender.verify_with(|_, _| {});
    let state = contender.state.clone();
    state.lock().unwrap().concurrent_receipt = Some(original.clone());
    assert_eq!(
        contender
            .service()
            .submit(
                "session",
                &consent(),
                &pdf_signature(),
                &consent_signature()
            )
            .unwrap(),
        original
    );
    assert_eq!(state.lock().unwrap().tsa_calls, 1);
}
