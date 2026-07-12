//! Behavior of the integral document verification use case when one
//! component at a time fails: each failure carries its cause, leaves
//! the other components untouched, and turns the verdict not valid.
//! The mocked ports live in `verification_mocks`.

mod verification_mocks;

use application::verification::{ComponentStatus, Verdict, VerifyDocument};
use domain::crypto::certificate::CertificateValidation;
use domain::crypto::timestamp::TimestampVerification;
use domain::crypto::{SignatureRejection, SignatureVerification};
use verification_mocks::{
    accepting_signature_verifier, full_request, hasher, signature, token_verifier_reporting,
    valid_token, validator_reporting, MockSigVerifier,
};

#[test]
fn a_rejected_signature_fails_only_the_signature_component() {
    let mut sig_verifier = MockSigVerifier::new();
    sig_verifier.expect_verify().times(1).returning(|_, _, _| {
        Ok(SignatureVerification::Invalid(
            SignatureRejection::MismatchedDocumentOrKey,
        ))
    });

    let use_case = VerifyDocument::new(
        hasher(),
        sig_verifier,
        validator_reporting(CertificateValidation::Valid),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"tampered body"[..], &full_request(&sig))
        .unwrap();

    assert_eq!(report.signature.status, ComponentStatus::Failed);
    assert!(
        report
            .signature
            .detail
            .contains("signature does not match document and key"),
        "the rejection cause must be reported: {}",
        report.signature.detail
    );
    assert_eq!(report.integrity.status, ComponentStatus::Passed);
    assert_eq!(report.certificate.status, ComponentStatus::Passed);
    assert_eq!(report.timestamp.status, ComponentStatus::Passed);
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn an_expired_certificate_fails_as_a_status_without_negating_the_signature() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_reporting(CertificateValidation::Expired),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &full_request(&sig))
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Failed);
    assert!(
        report.certificate.detail.contains("expired"),
        "the status must be named: {}",
        report.certificate.detail
    );
    assert!(
        report
            .certificate
            .detail
            .contains("does not by itself invalidate"),
        "the phrasing must leave the legal reading to the operator: {}",
        report.certificate.detail
    );
    assert_eq!(report.signature.status, ComponentStatus::Passed);
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn a_revoked_certificate_reports_the_matched_serial() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_reporting(CertificateValidation::Revoked {
            serial_hex: "10A3".to_string(),
        }),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &full_request(&sig))
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Failed);
    assert!(
        report.certificate.detail.contains("revoked (serial 10A3)"),
        "the matched serial must be reported: {}",
        report.certificate.detail
    );
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn an_untrusted_issuer_fails_the_certificate_component() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_reporting(CertificateValidation::UntrustedIssuer),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &full_request(&sig))
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Failed);
    assert!(report.certificate.detail.contains("untrusted issuer"));
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn an_imprint_mismatch_fails_integrity_and_timestamp() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_reporting(CertificateValidation::Valid),
        token_verifier_reporting(TimestampVerification::ImprintMismatch),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &full_request(&sig))
        .unwrap();

    assert_eq!(report.integrity.status, ComponentStatus::Failed);
    assert!(
        report.integrity.detail.contains("differs"),
        "the integrity cause must name the mismatch: {}",
        report.integrity.detail
    );
    assert_eq!(report.timestamp.status, ComponentStatus::Failed);
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn an_untrusted_token_fails_the_timestamp_but_not_the_integrity() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_reporting(CertificateValidation::Valid),
        token_verifier_reporting(TimestampVerification::UntrustedToken(
            "chain does not reach the anchor".to_string(),
        )),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &full_request(&sig))
        .unwrap();

    assert_eq!(report.integrity.status, ComponentStatus::Passed);
    assert_eq!(report.timestamp.status, ComponentStatus::Failed);
    assert!(
        report
            .timestamp
            .detail
            .contains("chain does not reach the anchor"),
        "the verifier diagnosis must be kept: {}",
        report.timestamp.detail
    );
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn a_malformed_token_fails_the_timestamp_and_leaves_integrity_without_reference() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_reporting(CertificateValidation::Valid),
        token_verifier_reporting(TimestampVerification::MalformedToken("not der".to_string())),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &full_request(&sig))
        .unwrap();

    assert_eq!(report.timestamp.status, ComponentStatus::Failed);
    assert!(report.timestamp.detail.contains("not der"));
    assert_eq!(report.integrity.status, ComponentStatus::Skipped);
    assert!(
        report.integrity.detail.contains("no reference digest"),
        "integrity must say the reference is missing: {}",
        report.integrity.detail
    );
    assert_eq!(report.verdict, Verdict::NotValid);
}
