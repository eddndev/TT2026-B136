use application::{
    credential_trust::CredentialTrustRevision, identity::owner_certificates::*, ApplicationError,
};
use domain::crypto::Signature;
use uuid::Uuid;

use crate::{submission_support::*, support::*};

#[test]
fn submission_replays_original_history_after_expiry_rotation_and_account_changes() {
    for terminal in [false, true] {
        let harness = harness();
        let observed = harness.state.clone();
        let stored = if terminal { retired() } else { receipt() };
        {
            let mut state = observed.lock().unwrap();
            state.found = Some(stored.clone());
            state.now = 5000;
            state.context.account.revision = 20;
            state.context.account.auth_generation = 12;
            state.context.trust.as_mut().unwrap().revision =
                CredentialTrustRevision::new(8).unwrap();
            state.principal.as_mut().unwrap().email = "current-owner@example.test".into();
            state.context.account.principal = state.principal.clone().unwrap();
        }
        let result = harness
            .service()
            .submit_registration("session", binding(), submission())
            .unwrap();
        assert_eq!(result, stored);
        assert_eq!(count(&observed, "load_registration"), 0);
        assert_eq!(count(&observed, "inspect"), 0);
        assert_eq!(count(&observed, "find"), 1);
        assert!(count(&observed, "authenticate") >= 2);
        no_crypto_or_commit(&observed);
    }
}

#[test]
fn historical_submission_compares_every_public_byte_and_requested_binding() {
    for variant in 0..4 {
        let mut harness = harness();
        let observed = harness.state.clone();
        let requested = if variant == 3 {
            Uuid::from_u128(45)
        } else {
            binding()
        };
        replace_store(
            &mut harness,
            requested,
            |_| {},
            |state| state.context.clone(),
        );
        observed.lock().unwrap().found = Some(receipt());
        let mut input = submission();
        match variant {
            0 => input.statement[0] ^= 1,
            1 => input.certificate_der[0] ^= 1,
            2 => input.signature = Signature::from_bytes(vec![6; 384]).unwrap(),
            _ => {}
        }
        let error = failure(
            harness
                .service()
                .submit_registration("session", requested, input),
        );
        assert_eq!(
            kind(&error),
            if variant == 3 {
                &OwnerCertificateError::Inconsistent
            } else {
                &OwnerCertificateError::BindingConflict
            }
        );
        assert_eq!(count(&observed, "load_registration"), 0);
        assert_eq!(count(&observed, "inspect"), 0);
        no_crypto_or_commit(&observed);
    }
}

#[test]
fn exact_new_submission_commits_only_server_prepared_and_verified_evidence() {
    let mut harness = harness();
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
    let result = harness
        .service()
        .submit_registration("session", binding(), submission())
        .unwrap();
    assert_eq!(result, receipt());
    assert_eq!(count(&observed, "verify"), 1);
    assert_eq!(count(&observed, "commit_registration"), 1);
    let calls = observed.lock().unwrap().calls.clone();
    assert!(
        calls.iter().position(|call| *call == "find").unwrap()
            < calls.iter().position(|call| *call == "inspect").unwrap()
    );
}

#[test]
fn absent_receipt_requires_exact_current_capture_and_canonical_der_before_crypto() {
    for variant in 0..6 {
        let mut harness = harness();
        let observed = harness.state.clone();
        let requested = if variant == 3 {
            Uuid::from_u128(45)
        } else {
            binding()
        };
        replace_store(
            &mut harness,
            requested,
            |_| {},
            |state| state.context.clone(),
        );
        let mut input = submission();
        match variant {
            0 => observed.lock().unwrap().context.account.revision += 1,
            1 => observed.lock().unwrap().context.account.auth_generation += 1,
            2 => {
                observed
                    .lock()
                    .unwrap()
                    .context
                    .trust
                    .as_mut()
                    .unwrap()
                    .revision = CredentialTrustRevision::new(8).unwrap()
            }
            4 => input.statement[8] = 2,
            5 => {
                input.certificate_der = RAW_CERTIFICATE.to_vec();
                harness.verifier = MockVerifier::new();
                let calls = observed.clone();
                harness
                    .verifier
                    .expect_inspect_certificate()
                    .times(1)
                    .returning(move |bytes| {
                        assert_eq!(bytes, RAW_CERTIFICATE);
                        calls.lock().unwrap().calls.push("inspect");
                        Ok(certificate())
                    });
            }
            _ => {}
        }
        let error = failure(
            harness
                .service()
                .submit_registration("session", requested, input),
        );
        assert_eq!(kind(&error), &OwnerCertificateError::BindingConflict);
        no_crypto_or_commit(&observed);
    }
}

#[test]
fn submission_lengths_and_nil_identity_reject_before_loading_or_inspecting_material() {
    for variant in 0..8 {
        let harness = harness();
        let observed = harness.state.clone();
        let mut input = submission();
        let mut requested = binding();
        match variant {
            0 => input.statement.clear(),
            1 => {
                input.statement.pop();
            }
            2 => input.statement.push(0),
            3 => input.certificate_der.clear(),
            4 => input.certificate_der = vec![0; 16 * 1024 + 1],
            5 => input.signature = Signature::from_bytes(vec![1; 383]).unwrap(),
            6 => input.signature = Signature::from_bytes(vec![1; 385]).unwrap(),
            _ => requested = Uuid::nil(),
        }
        let error = failure(
            harness
                .service()
                .submit_registration("session", requested, input),
        );
        assert!(matches!(
            error,
            ApplicationError::OwnerCertificate(OwnerCertificateError::InvalidInput)
        ));
        assert_eq!(count(&observed, "find"), 0);
        assert_eq!(count(&observed, "load_registration"), 0);
        assert_eq!(count(&observed, "inspect"), 0);
        no_crypto_or_commit(&observed);
    }
}
