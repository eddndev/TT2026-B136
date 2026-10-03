use application::{identity::owner_certificates::*, ApplicationError};
use domain::{
    crypto::{CredentialFailure, Signature},
    identity::Role,
    owner_certificates::{BindingRecord, BindingStatement, OwnerAccount},
};

use crate::support::*;

#[test]
fn exact_same_uuid_receipt_survives_retirement_and_expiry_but_changed_bytes_conflict() {
    for variant in 0..5 {
        let harness = Harness::new();
        let observed = harness.state.clone();
        let service = harness.service();
        let prepared = prepare(&service);
        let mut stored = if variant == 1 { retired() } else { receipt() };
        if variant == 3 {
            stored.check.certificate.der.push(0);
        }
        if variant == 4 {
            let altered = BindingStatement::new(
                OwnerAccount::new(principal().id, Role::Owner, true, 10, 4).unwrap(),
                principal().id,
                *statement().material(),
            )
            .unwrap();
            stored.check = check(&altered, &signature(), &trust(), 1000);
            stored.record = BindingRecord::registered(altered);
        }
        {
            let mut state = observed.lock().unwrap();
            state.found = Some(stored.clone());
            state.context.trust = None;
            state.now = 4000;
            state.calls.clear();
        }
        let signature = if variant == 2 {
            Signature::from_bytes(vec![6; 384]).unwrap()
        } else {
            signature()
        };
        let result = service.register("session", &prepared, &signature);
        if variant < 2 {
            let result = result.unwrap();
            assert_eq!(result.record, stored.record);
            assert_eq!(result.registered_at, stored.registered_at);
            assert_eq!(result.withdrawn_at, stored.withdrawn_at);
            assert_eq!(
                observed.lock().unwrap().calls,
                ["authenticate", "find", "authenticate"]
            );
        } else {
            assert_eq!(
                kind(&failure(result)),
                &OwnerCertificateError::BindingConflict
            );
            assert_eq!(observed.lock().unwrap().calls, ["authenticate", "find"]);
        }
    }
}

#[test]
fn commit_rejections_and_uncertainty_propagate_without_a_second_attempt() {
    for variant in 0..7 {
        let mut harness = Harness::new();
        harness.verify_with(|_, _| {});
        let observed = harness.state.clone();
        if variant == 6 {
            observed.lock().unwrap().now = 1100;
        }
        let calls = observed.clone();
        harness
            .store
            .expect_commit_registration()
            .times(1)
            .returning(move |command| {
                calls.lock().unwrap().calls.push("commit_registration");
                assert_eq!(command.registration().account().auth_generation, 4);
                assert_eq!(
                    command.not_before(),
                    at(if variant == 6 { 1100 } else { 1000 })
                );
                if variant == 6 {
                    return Ok(OwnerBindingCommit::Existing(receipt()));
                }
                Err(match variant {
                    0 => OwnerCertificateError::ActiveBinding.into(),
                    1 => OwnerCertificateError::FingerprintConflict.into(),
                    2 => OwnerCertificateError::TrustChanged.into(),
                    3 => OwnerCertificateError::AccountChanged.into(),
                    4 => OwnerCertificateError::Credential(CredentialFailure::Expired).into(),
                    _ => ApplicationError::Port("controlled uncertain commit".into()),
                })
            });
        let service = harness.service();
        let prepared = prepare(&service);
        observed.lock().unwrap().calls.clear();
        let result = service.register("session", &prepared, &signature());
        if variant == 6 {
            assert_eq!(result.unwrap().registered_at, at(1000));
        } else {
            assert!(matches!(
                failure(result),
                ApplicationError::OwnerCertificate(_) | ApplicationError::Port(_)
            ));
        }
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
}
