use crate::{
    document_signing_support::{self as bindings, failure, kind},
    individual_document_support::*,
};
use application::{document_signing::SigningWorkflowError as Error, ApplicationError};
use domain::{
    crypto::{CredentialFailure, Sha256Digest, Signature},
    document_signing::SignerAccount,
    identity::Role,
};

#[test]
fn prepare_denies_inactive_binding_non_pdf_and_mismatched_document_before_timestamp() {
    let harness = Harness::new();
    harness.binding.state.lock().unwrap().found = None;
    assert_eq!(
        kind(failure(harness.service().prepare(
            "session",
            operation(),
            case(),
            reference(),
            &pdf_signature()
        ))),
        Error::InactiveBinding
    );
    for change in 0..3 {
        let harness = Harness::new();
        let state = harness.state.clone();
        {
            let mut value = state.lock().unwrap();
            match change {
                0 => value.docx = true,
                1 => value.record.version = domain::crypto::DocumentVersion::new(2).unwrap(),
                _ => value.record.digest = Sha256Digest::from_array([1; 32]),
            }
        }
        assert!(harness
            .service()
            .prepare(
                "session",
                operation(),
                case(),
                reference(),
                &pdf_signature()
            )
            .is_err());
        assert_eq!(state.lock().unwrap().tsa_calls, 0);
    }
}

#[test]
fn changed_account_trust_and_permissions_reject_submission_before_verifier_or_tsa() {
    for change in 0..4 {
        let harness = Harness::new();
        let state = harness.state.clone();
        {
            let mut value = harness.binding.state.lock().unwrap();
            match change {
                0 => {
                    value.context.account =
                        SignerAccount::new(bindings::principal().id, Role::Litigator, true, 10, 4)
                            .unwrap()
                }
                1 => {
                    value.context.trust.as_mut().unwrap().revision =
                        application::credential_trust::CredentialTrustRevision::new(8).unwrap()
                }
                2 => value.principal.as_mut().unwrap().role = Role::Owner,
                _ => state.lock().unwrap().deny_document = true,
            }
        }
        assert!(harness
            .service()
            .submit(
                "session",
                &consent(),
                &pdf_signature(),
                &consent_signature()
            )
            .is_err());
        let value = state.lock().unwrap();
        assert_eq!((value.tsa_calls, value.commits), (0, 0));
    }
}

#[test]
fn substituted_verifier_evidence_and_expiration_during_crypto_never_commit() {
    for change in 0..4 {
        let mut harness = Harness::new();
        let state = harness.state.clone();
        harness.verify_with(move |check, authority| match change {
            0 => check.statement_digest = Sha256Digest::from_array([0; 32]),
            1 => check.signature = Signature::from_bytes(vec![9; 384]).unwrap(),
            2 => authority.lock().unwrap().now = 2001,
            _ => {
                authority.lock().unwrap().principal.as_mut().unwrap().email =
                    "other@example.test".into()
            }
        });
        assert!(harness
            .service()
            .submit(
                "session",
                &consent(),
                &pdf_signature(),
                &consent_signature()
            )
            .is_err());
        assert_eq!(state.lock().unwrap().commits, 0);
    }
}

#[test]
fn revoked_proof_timeout_and_bad_timestamp_never_commit_or_retry() {
    let mut harness = Harness::new();
    let state = harness.state.clone();
    harness
        .binding
        .verifier
        .expect_verify_consent()
        .times(1)
        .returning(|_, _, _, _, _, _| Err(Error::Credential(CredentialFailure::Revoked)));
    assert_eq!(
        kind(failure(harness.service().submit(
            "session",
            &consent(),
            &pdf_signature(),
            &consent_signature()
        ))),
        Error::Credential(CredentialFailure::Revoked)
    );
    assert_eq!(state.lock().unwrap().tsa_calls, 0);
    for timeout in [true, false] {
        let mut harness = Harness::new();
        harness.verify_with(|_, _| {});
        let state = harness.state.clone();
        {
            let mut value = state.lock().unwrap();
            value.timeout = timeout;
            value.bad_timestamp = !timeout;
        }
        assert!(harness
            .service()
            .submit(
                "session",
                &consent(),
                &pdf_signature(),
                &consent_signature()
            )
            .is_err());
        let value = state.lock().unwrap();
        assert_eq!((value.tsa_calls, value.commits), (1, 0));
    }
}

#[test]
fn malformed_signature_and_corrupt_committed_receipt_are_rejected() {
    let harness = Harness::new();
    assert_eq!(
        kind(failure(harness.service().prepare(
            "session",
            operation(),
            case(),
            reference(),
            &Signature::from_bytes(vec![1]).unwrap()
        ))),
        Error::Input
    );
    let mut harness = Harness::new();
    harness.verify_with(|_, _| {});
    harness.state.lock().unwrap().corrupt_receipt = true;
    assert!(matches!(
        failure(harness.service().submit(
            "session",
            &consent(),
            &pdf_signature(),
            &consent_signature()
        )),
        ApplicationError::DocumentSigning(_)
    ));
}
