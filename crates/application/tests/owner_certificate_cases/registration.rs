use application::{identity::owner_certificates::*, ApplicationError};
use domain::crypto::{CredentialFailure, Sha256Digest, Signature};

use crate::support::*;

#[test]
fn new_registration_passes_only_exact_verified_public_evidence_to_one_commit() {
    let mut harness = Harness::new();
    let observed = harness.state.clone();
    harness.verify_with(|_, _| {});
    let calls = observed.clone();
    harness
        .store
        .expect_commit_registration()
        .times(1)
        .returning(move |command| {
            calls.lock().unwrap().calls.push("commit_registration");
            assert_eq!(command.registration().account(), &account());
            assert_eq!(command.registration().statement(), &statement());
            assert_eq!(command.registration().certificate(), &certificate());
            assert_eq!(command.registration().trust(), &trust());
            assert_eq!(
                command.check(),
                &check(&statement(), &signature(), &trust(), 1000)
            );
            assert_eq!(command.not_before(), at(1000));
            Ok(OwnerBindingCommit::Applied(receipt()))
        });
    let service = harness.service();
    let prepared = prepare(&service);
    observed.lock().unwrap().calls.clear();
    let result = service
        .register("session", &prepared, &signature())
        .unwrap();
    assert_eq!(result.record, receipt().record);
    assert_eq!(result.check, receipt().check);
    assert_eq!(
        observed.lock().unwrap().calls,
        [
            "authenticate",
            "find",
            "load_registration",
            "verify",
            "authenticate",
            "commit_registration"
        ]
    );
}

#[test]
fn changed_account_or_whole_trust_rejects_before_verification_and_commit() {
    for variant in 0..4 {
        let harness = Harness::new();
        let observed = harness.state.clone();
        let service = harness.service();
        let prepared = prepare(&service);
        {
            let mut state = observed.lock().unwrap();
            state.calls.clear();
            match variant {
                0 => state.context.account.revision += 1,
                1 => state.context.account.auth_generation += 1,
                2 => {
                    state.context.trust.as_mut().unwrap().inspection.crl_digest =
                        Sha256Digest::from_array([8; 32])
                }
                _ => state.context.trust.as_mut().unwrap().published_at = at(101),
            }
        }
        let error = failure(service.register("session", &prepared, &signature()));
        assert_eq!(
            kind(&error),
            if variant < 2 {
                &OwnerCertificateError::AccountChanged
            } else {
                &OwnerCertificateError::TrustChanged
            }
        );
        assert_eq!(
            observed.lock().unwrap().calls,
            ["authenticate", "find", "load_registration"]
        );
    }
}

#[test]
fn forged_verification_fields_never_become_a_prepared_commit() {
    for variant in 0..7 {
        let mut harness = Harness::new();
        if variant == 6 {
            let calls = harness.state.clone();
            harness
                .verifier
                .expect_verify_registration()
                .times(1)
                .returning(move |_, _, _, _, _| {
                    calls.lock().unwrap().calls.push("verify");
                    Err(OwnerCertificateError::Credential(
                        CredentialFailure::Revoked,
                    ))
                });
        } else {
            harness.verify_with(move |check, _| match variant {
                0 => check.statement_digest = Sha256Digest::from_array([9; 32]),
                1 => check.certificate.der.push(0),
                2 => check.signature = Signature::from_bytes(vec![6; 384]).unwrap(),
                3 => check.trust.crl_number += 1,
                4 => check.checked_at -= 1,
                _ => check.valid_until += 1,
            });
        }
        let observed = harness.state.clone();
        let service = harness.service();
        let prepared = prepare(&service);
        observed.lock().unwrap().calls.clear();
        let error = failure(service.register("session", &prepared, &signature()));
        assert_eq!(
            kind(&error),
            if variant == 6 {
                &OwnerCertificateError::Credential(CredentialFailure::Revoked)
            } else {
                &OwnerCertificateError::Inconsistent
            }
        );
        assert_eq!(
            observed.lock().unwrap().calls,
            ["authenticate", "find", "load_registration", "verify"]
        );
    }
}

#[test]
fn time_or_full_principal_changes_during_verification_cannot_reach_commit() {
    for variant in 0..4 {
        let mut harness = Harness::new();
        harness.verify_with(move |_, observed| {
            let mut state = observed.lock().unwrap();
            match variant {
                0 => state.now = 2001,
                1 => state.now = 999,
                2 => state.principal = None,
                _ => state.principal.as_mut().unwrap().email = "changed@example.test".into(),
            }
        });
        let service = harness.service();
        let prepared = prepare(&service);
        let error = failure(service.register("session", &prepared, &signature()));
        match variant {
            0 => assert_eq!(
                kind(&error),
                &OwnerCertificateError::Credential(CredentialFailure::Expired)
            ),
            1 => assert_eq!(kind(&error), &OwnerCertificateError::Inconsistent),
            _ => assert!(matches!(error, ApplicationError::InvalidSession)),
        }
    }
}
