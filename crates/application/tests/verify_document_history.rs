//! Behavior of the integral document verification use case over time:
//! the certificate is judged at the signing time a verified timestamp
//! token establishes, with the supplied revocation list, while its status
//! at the evaluation time is reported only as an informative note. Without
//! a verified token there is no trusted signing time, and a revocation
//! list that no longer covers the evaluation time is an outcome inside the
//! report, never an error.

use crate::verification_mocks;
use application::verification::{ComponentStatus, Verdict, VerifyDocument, VerifyDocumentRequest};
use domain::crypto::certificate::CertificateValidation;
use domain::crypto::timestamp::TimestampVerification;
use domain::DomainError;
use verification_mocks::{
    accepting_signature_verifier, full_request, hasher, signature, token_verifier_reporting,
    unused_token_verifier, valid_token, validator_answering, AFTER_TOKEN_UNIX, TOKEN_UNIX,
};

/// Next update of a list issued shortly before the token: still current at
/// the token time, already past at [`AFTER_TOKEN_UNIX`].
const LIST_NEXT_UPDATE_UNIX: i64 = TOKEN_UNIX + 7 * 86_400;

/// Mirrors the certificate validator port: a supplied list past its next
/// update is an error, and the window check answers before the list.
fn list_expiring_after_the_token(
    window: impl Fn(i64) -> CertificateValidation + Send + 'static,
) -> verification_mocks::MockCertValidator {
    validator_answering(move |at, with_list| {
        let status = window(at);
        if status != CertificateValidation::Valid || !with_list {
            return Ok(status);
        }
        if at > LIST_NEXT_UPDATE_UNIX {
            return Err(DomainError::StaleCrl {
                next_update_unix: LIST_NEXT_UPDATE_UNIX,
            });
        }
        Ok(CertificateValidation::Valid)
    })
}

fn later_request(sig: &domain::crypto::Signature) -> VerifyDocumentRequest<'_> {
    VerifyDocumentRequest {
        evaluation_unix: AFTER_TOKEN_UNIX,
        ..full_request(sig)
    }
}

#[test]
fn a_list_past_its_next_update_today_still_judges_the_certificate_at_the_token_time() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        list_expiring_after_the_token(|_| CertificateValidation::Valid),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &later_request(&sig))
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Passed);
    let detail = &report.certificate.detail;
    assert!(
        detail.contains("signing time 2026-07-12T00:00:00Z"),
        "the judged instant must be the token time: {detail}"
    );
    assert!(
        detail.contains("informative") && detail.contains("next update"),
        "today's status must be a note about the list that no longer covers it: {detail}"
    );
    assert_eq!(report.verdict, Verdict::Valid);
}

#[test]
fn a_revocation_listed_at_the_token_time_fails_the_certificate() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_answering(|at, with_list| {
            if at == TOKEN_UNIX && with_list {
                Ok(CertificateValidation::Revoked {
                    serial_hex: "10A3".to_string(),
                })
            } else {
                Err(DomainError::StaleCrl {
                    next_update_unix: LIST_NEXT_UPDATE_UNIX,
                })
            }
        }),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &later_request(&sig))
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Failed);
    assert!(
        report.certificate.detail.contains("revoked (serial 10A3)"),
        "the revocation must be named: {}",
        report.certificate.detail
    );
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn a_certificate_expired_today_but_valid_at_the_token_time_passes_with_a_note() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        list_expiring_after_the_token(|at| {
            if at > TOKEN_UNIX + 86_400 {
                CertificateValidation::Expired
            } else {
                CertificateValidation::Valid
            }
        }),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &later_request(&sig))
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Passed);
    assert!(
        report.certificate.detail.contains("informative")
            && report.certificate.detail.contains("expired"),
        "today's expiry must appear only as a note: {}",
        report.certificate.detail
    );
    assert_eq!(report.verdict, Verdict::Valid);
}

#[test]
fn a_list_already_past_its_next_update_at_the_token_time_fails_the_certificate() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_answering(|_, _| {
            Err(DomainError::StaleCrl {
                next_update_unix: TOKEN_UNIX - 86_400,
            })
        }),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &later_request(&sig))
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Failed);
    assert!(
        report.certificate.detail.contains("next update"),
        "the cause must name the list update: {}",
        report.certificate.detail
    );
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn without_a_token_a_list_past_its_next_update_skips_the_certificate() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        list_expiring_after_the_token(|_| CertificateValidation::Valid),
        unused_token_verifier(),
    );
    let sig = signature();
    let request = VerifyDocumentRequest {
        timestamp: None,
        ..later_request(&sig)
    };
    let report = use_case
        .execute(&mut &b"document body"[..], &request)
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Skipped);
    let detail = &report.certificate.detail;
    assert!(
        detail.contains("no verified timestamp token") && detail.contains("next update"),
        "the skip reason must be explicit: {detail}"
    );
    // Skipped components are reported but never count against the verdict.
    assert_eq!(report.verdict, Verdict::Valid);
}

#[test]
fn an_untrusted_token_gives_no_signing_time_and_a_stale_list_is_not_an_error() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        list_expiring_after_the_token(|_| CertificateValidation::Valid),
        token_verifier_reporting(TimestampVerification::UntrustedToken(
            "chain does not reach the anchor".to_string(),
        )),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &later_request(&sig))
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Skipped);
    assert_eq!(report.timestamp.status, ComponentStatus::Failed);
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn a_recorded_signing_time_replaces_the_token_time_when_the_token_verifies() {
    const RECORDED_UNIX: i64 = TOKEN_UNIX - 60;
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_answering(|at, with_list| {
            assert!(with_list, "every call carries the supplied list");
            assert!(
                at == RECORDED_UNIX || at == AFTER_TOKEN_UNIX,
                "unexpected evaluation instant {at}"
            );
            Ok(CertificateValidation::Valid)
        }),
        token_verifier_reporting(valid_token()),
    );
    let sig = signature();
    let request = VerifyDocumentRequest {
        recorded_signing_unix: Some(RECORDED_UNIX),
        ..later_request(&sig)
    };
    let report = use_case
        .execute(&mut &b"document body"[..], &request)
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Passed);
    assert!(
        report.certificate.detail.contains("2026-07-11T23:59:00Z"),
        "the recorded instant must be named: {}",
        report.certificate.detail
    );
    assert!(
        report
            .certificate
            .detail
            .contains("recorded with the signature evidence")
            && !report.certificate.detail.contains("covered by"),
        "the origin must not claim that the token dates the recorded instant: {}",
        report.certificate.detail
    );
}

#[test]
fn a_recorded_signing_time_is_not_trusted_without_a_verified_token() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        validator_answering(|at, _| {
            assert_eq!(at, AFTER_TOKEN_UNIX, "only the evaluation time is usable");
            Ok(CertificateValidation::Valid)
        }),
        token_verifier_reporting(TimestampVerification::ImprintMismatch),
    );
    let sig = signature();
    let request = VerifyDocumentRequest {
        recorded_signing_unix: Some(TOKEN_UNIX),
        ..later_request(&sig)
    };
    let report = use_case
        .execute(&mut &b"document body"[..], &request)
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Passed);
    assert_eq!(report.verdict, Verdict::NotValid);
}

#[test]
fn a_verified_token_with_an_unreadable_generation_time_fails_the_certificate() {
    let use_case = VerifyDocument::new(
        hasher(),
        accepting_signature_verifier(),
        verification_mocks::MockCertValidator::new(),
        token_verifier_reporting(TimestampVerification::Valid {
            generated_at: "unknown time".to_string(),
        }),
    );
    let sig = signature();
    let report = use_case
        .execute(&mut &b"document body"[..], &later_request(&sig))
        .unwrap();

    assert_eq!(report.certificate.status, ComponentStatus::Failed);
    assert!(
        report.certificate.detail.contains("cannot be read"),
        "the cause must be named: {}",
        report.certificate.detail
    );
    assert_eq!(report.verdict, Verdict::NotValid);
}
