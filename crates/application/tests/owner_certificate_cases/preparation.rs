use application::{identity::owner_certificates::OwnerCertificateError, ApplicationError};
use domain::{
    crypto::Signature,
    identity::{Role, UserId},
};
use uuid::Uuid;

use crate::support::*;

#[test]
fn preparation_binds_current_owner_and_published_material_without_writing() {
    let harness = Harness::new();
    let observed = harness.state.clone();
    let service = harness.service();
    let prepared = prepare(&service);
    assert_eq!(prepared.account(), &account());
    assert_eq!(
        prepared.statement().canonical_bytes(),
        statement().canonical_bytes()
    );
    assert_eq!(prepared.statement().canonical_bytes().len(), 150);
    assert_eq!(prepared.certificate(), &certificate());
    assert_eq!(prepared.trust(), &trust());
    assert_eq!(
        observed.lock().unwrap().calls,
        [
            "authenticate",
            "inspect",
            "load_registration",
            "authenticate"
        ]
    );
}

#[test]
fn nonowners_and_invalid_public_limits_are_denied_before_lookup_or_crypto() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let harness = Harness::new();
        let observed = harness.state.clone();
        observed.lock().unwrap().principal.as_mut().unwrap().role = role;
        let error = failure(harness.service().prepare_registration(
            "session",
            binding(),
            RAW_CERTIFICATE,
        ));
        assert!(matches!(error, ApplicationError::PermissionDenied));
        assert_eq!(observed.lock().unwrap().calls, ["authenticate"]);
    }
    for (id, certificate) in [
        (Uuid::nil(), RAW_CERTIFICATE.to_vec()),
        (binding(), vec![0; 16 * 1024 + 1]),
        (binding(), Vec::new()),
    ] {
        let harness = Harness::new();
        let observed = harness.state.clone();
        let error = failure(
            harness
                .service()
                .prepare_registration("session", id, &certificate),
        );
        assert_eq!(kind(&error), &OwnerCertificateError::InvalidInput);
        assert_eq!(observed.lock().unwrap().calls, ["authenticate"]);
    }
    for length in [1, 383, 385, 16 * 1024] {
        let harness = Harness::new();
        let observed = harness.state.clone();
        let service = harness.service();
        let prepared = prepare(&service);
        observed.lock().unwrap().calls.clear();
        let signature = Signature::from_bytes(vec![0; length]).unwrap();
        let error = failure(service.register("session", &prepared, &signature));
        assert_eq!(kind(&error), &OwnerCertificateError::InvalidInput);
        assert_eq!(observed.lock().unwrap().calls, ["authenticate"]);
    }
}

#[test]
fn preparation_rejects_missing_trust_or_inconsistent_current_account() {
    for variant in 0..7 {
        let harness = Harness::new();
        let observed = harness.state.clone();
        {
            let mut state = observed.lock().unwrap();
            match variant {
                0 => state.context.trust = None,
                1 => state.context.account.active = false,
                2 => state.context.account.principal.role = Role::Litigator,
                3 => state.context.account.principal.id = UserId::from_uuid(Uuid::from_u128(2)),
                4 => state.context.account.principal.email = "other@example.test".into(),
                5 => state.context.account.auth_generation = 10,
                _ => state.context.account.revision = i64::MAX as u64 + 1,
            }
        }
        let error = failure(harness.service().prepare_registration(
            "session",
            binding(),
            RAW_CERTIFICATE,
        ));
        if variant == 0 {
            assert_eq!(kind(&error), &OwnerCertificateError::TrustUnavailable);
        } else {
            assert!(matches!(error, ApplicationError::OwnerCertificate(_)));
        }
        assert_eq!(
            observed.lock().unwrap().calls,
            ["authenticate", "inspect", "load_registration"]
        );
    }
}
