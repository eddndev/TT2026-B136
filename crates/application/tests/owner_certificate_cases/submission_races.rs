use application::{
    credential_trust::CredentialTrustRevision, identity::owner_certificates::*, ApplicationError,
};

use crate::{submission_support::*, support::*};

#[test]
fn publication_rotation_after_reconstruction_rejects_without_reinterpreting_intent() {
    for during_verification in [false, true] {
        let mut harness = harness();
        let observed = harness.state.clone();
        if during_verification {
            harness.verify_with(|_, state| {
                state
                    .lock()
                    .unwrap()
                    .context
                    .trust
                    .as_mut()
                    .unwrap()
                    .revision = CredentialTrustRevision::new(8).unwrap();
            });
            let calls = observed.clone();
            harness
                .store
                .expect_commit_registration()
                .times(1)
                .returning(move |command| {
                    let mut state = calls.lock().unwrap();
                    state.calls.push("commit_registration");
                    assert_eq!(command.registration().trust(), &trust());
                    assert_ne!(
                        Some(command.registration().trust()),
                        state.context.trust.as_ref()
                    );
                    Err(OwnerCertificateError::TrustChanged.into())
                });
        } else {
            replace_store(
                &mut harness,
                binding(),
                |_| {},
                |state| {
                    let captured = state.context.clone();
                    state.context.trust.as_mut().unwrap().revision =
                        CredentialTrustRevision::new(8).unwrap();
                    captured
                },
            );
        }
        let error = failure(harness.service().submit_registration(
            "session",
            binding(),
            submission(),
        ));
        assert_eq!(kind(&error), &OwnerCertificateError::TrustChanged);
        assert_eq!(count(&observed, "verify"), usize::from(during_verification));
        assert_eq!(
            count(&observed, "commit_registration"),
            usize::from(during_verification)
        );
    }
}

#[test]
fn changed_full_principal_between_receipt_and_nested_preparation_cannot_commit() {
    for found in [false, true] {
        let mut harness = harness();
        let observed = harness.state.clone();
        if found {
            observed.lock().unwrap().found = Some(receipt());
        }
        replace_store(
            &mut harness,
            binding(),
            |state| {
                state.principal.as_mut().unwrap().email = "replaced-owner@example.test".into();
                state.context.account.principal = state.principal.clone().unwrap();
            },
            |state| state.context.clone(),
        );
        let error = failure(harness.service().submit_registration(
            "session",
            binding(),
            submission(),
        ));
        assert!(matches!(error, ApplicationError::InvalidSession));
        no_crypto_or_commit(&observed);
    }
}

#[test]
fn submission_uncertain_commit_is_once_and_concurrent_terminal_receipt_stays_original() {
    for existing in [false, true] {
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
                assert_eq!(command.registration().statement(), &statement());
                assert_eq!(command.registration().certificate(), &certificate());
                if existing {
                    Ok(OwnerBindingCommit::Existing(retired()))
                } else {
                    Err(ApplicationError::Port("controlled uncertain commit".into()))
                }
            });
        let result = harness
            .service()
            .submit_registration("session", binding(), submission());
        if existing {
            assert_eq!(result.unwrap(), retired());
        } else {
            assert!(matches!(failure(result), ApplicationError::Port(_)));
        }
        assert_eq!(count(&observed, "verify"), 1);
        assert_eq!(count(&observed, "commit_registration"), 1);
    }
}
