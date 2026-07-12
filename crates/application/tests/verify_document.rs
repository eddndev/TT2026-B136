//! Behavior of the integral document verification use case on its
//! passing paths: a fully valid report, honest reporting of skipped
//! components, and propagation of hard errors. The mocked ports live
//! in `verification_mocks`; the per-component failure cases live in
//! `verify_document_failures.rs`.

mod verification_mocks;

use application::verification::{ComponentStatus, Verdict, VerifyDocument, VerifyDocumentRequest};
use application::ApplicationError;
use domain::crypto::certificate::CertificateValidation;
use domain::crypto::SignatureVerification;
use domain::DomainError;
use verification_mocks::{
    accepting_signature_verifier, digest, full_request, hasher, signature,
    token_verifier_reporting, unused_token_verifier, valid_token, validator_reporting,
    MockCertValidator, MockHasher, MockSigVerifier, MockTsVerifier, EVALUATION_UNIX,
};

#[test]
fn all_supplied_components_passing_yield_a_valid_verdict() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_reporting(CertificateValidation::Valid),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &full_request(&sig))
        .unwrap();

    assert_eq!(report.document_digest_hex, digest().to_hex());
    assert_eq!(report.integrity.status, ComponentStatus::Passed);
    assert_eq!(report.signature.status, ComponentStatus::Passed);
    assert_eq!(report.certificate.status, ComponentStatus::Passed);
    assert_eq!(report.timestamp.status, ComponentStatus::Passed);
    assert!(
        report.timestamp.detail.contains("2026-07-12T00:00:00Z"),
        "the timestamp detail must carry the generation time: {}",
        report.timestamp.detail
    );
    assert!(
        report.certificate.detail.contains("revocation checked"),
        "with a revocation list the certificate detail must say so: {}",
        report.certificate.detail
    );
    assert_eq!(report.verdict, Verdict::Valid);
}

#[test]
fn ports_receive_the_digest_signature_and_supplied_material() {
    let mut sig_verifier = MockSigVerifier::new();
    sig_verifier
        .expect_verify()
        .withf(|d, s, material| *d == digest() && *s == signature() && material == b"signer pem")
        .times(1)
        .returning(|_, _, _| Ok(SignatureVerification::Valid));

    let mut validator = MockCertValidator::new();
    validator
        .expect_validate()
        .withf(|cert, issuer, crl, at| {
            cert == b"signer pem"
                && issuer == b"issuer pem"
                && crl == &Some(&b"crl pem"[..])
                && *at == EVALUATION_UNIX
        })
        .times(1)
        .returning(|_, _, _, _| Ok(CertificateValidation::Valid));

    let mut token_verifier = MockTsVerifier::new();
    token_verifier
        .expect_verify()
        .withf(|token, expected, anchor| {
            token == b"token der" && *expected == digest() && anchor == b"anchor pem"
        })
        .times(1)
        .returning(|_, _, _| Ok(valid_token()));

    let use_case = VerifyDocument::new(hasher(), sig_verifier, validator, token_verifier);
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &full_request(&sig))
        .unwrap();
    assert_eq!(report.verdict, Verdict::Valid);
}

#[test]
fn without_a_token_integrity_and_timestamp_are_skipped_with_reasons() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_reporting(CertificateValidation::Valid),
        unused_token_verifier(),
    );
    let sig = signature();
    let request = VerifyDocumentRequest {
        timestamp: None,
        ..full_request(&sig)
    };
    let report = use_case
        .execute(&mut &b"document body"[..], &request)
        .unwrap();

    assert_eq!(report.integrity.status, ComponentStatus::Skipped);
    assert!(
        report.integrity.detail.contains("digest recomputed")
            && report.integrity.detail.contains("no independent reference"),
        "integrity must report what it checked: {}",
        report.integrity.detail
    );
    assert_eq!(report.timestamp.status, ComponentStatus::Skipped);
    assert!(
        report.timestamp.detail.contains("no timestamp token"),
        "the skip reason must be explicit: {}",
        report.timestamp.detail
    );
    assert_eq!(report.verdict, Verdict::Valid);
}

#[test]
fn without_a_revocation_list_the_certificate_detail_says_so() {
    let mut validator = MockCertValidator::new();
    validator
        .expect_validate()
        .withf(|_, _, crl, _| crl.is_none())
        .times(1)
        .returning(|_, _, _, _| Ok(CertificateValidation::Valid));

    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator,
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let request = VerifyDocumentRequest {
        crl_pem: None,
        ..full_request(&sig)
    };
    let report = use_case
        .execute(&mut &b"document body"[..], &request)
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Passed);
    assert!(
        report.certificate.detail.contains("revocation not checked"),
        "the unchecked revocation must be reported: {}",
        report.certificate.detail
    );
    assert_eq!(report.verdict, Verdict::Valid);
}

#[test]
fn a_stale_revocation_list_is_a_hard_error_not_a_report() {
    let mut validator = MockCertValidator::new();
    validator
        .expect_validate()
        .times(1)
        .returning(|_, _, _, _| {
            Err(DomainError::StaleCrl {
                next_update_unix: 100,
            })
        });

    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator,
        unused_token_verifier(),
    );
    let sig = signature();
    let err = use_case
        .execute(&mut &b"document body"[..], &full_request(&sig))
        .unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::Domain(DomainError::StaleCrl { .. })
    ));
}

#[test]
fn a_stream_read_failure_stops_before_any_verification_port_runs() {
    let mut hasher = MockHasher::new();
    hasher.expect_hash_stream().times(1).returning(|_| {
        Err(DomainError::StreamRead {
            message: "broken pipe".to_string(),
        })
    });
    let mut sig_verifier = MockSigVerifier::new();
    sig_verifier.expect_verify().times(0);
    let mut validator = MockCertValidator::new();
    validator.expect_validate().times(0);

    let use_case = VerifyDocument::new(hasher, sig_verifier, validator, unused_token_verifier());
    let sig = signature();
    let err = use_case
        .execute(&mut std::io::empty(), &full_request(&sig))
        .unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::Domain(DomainError::StreamRead { .. })
    ));
}
