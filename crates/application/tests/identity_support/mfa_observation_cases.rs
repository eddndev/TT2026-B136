use super::identity_support::{service, MemorySessions, MemoryUsers};
use application::{identity::MfaReason, ApplicationError};
use domain::identity::Role;
use std::sync::Arc;

#[test]
fn accepted_invalid_replayed_and_missing_challenges_have_distinct_observations() {
    let service = service(
        Arc::new(MemoryUsers::default()),
        Arc::new(MemorySessions::default()),
    );
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let login = || {
        service
            .start_login("owner@example.com", "correct horse battery")
            .unwrap()
    };
    let invalid = service.complete_totp_observed(&login().challenge_token, "000000");
    assert_eq!(invalid.reason, MfaReason::InvalidCodeOrOutsideWindow);
    assert_eq!(invalid.user_id, Some(owner.principal.id));
    assert!(matches!(invalid.result, Err(ApplicationError::MfaRejected)));
    let challenge = login();
    let accepted = service.complete_totp_observed(&challenge.challenge_token, "123456");
    assert_eq!(accepted.reason, MfaReason::Accepted);
    assert!(accepted.result.is_ok());
    let replay = service.complete_totp_observed(&login().challenge_token, "123456");
    assert_eq!(replay.reason, MfaReason::CodeAlreadyUsed);
    assert!(matches!(replay.result, Err(ApplicationError::MfaRejected)));
    for token in [&challenge.challenge_token, "unknown"] {
        let missing = service.complete_totp_observed(token, "123456");
        assert_eq!(missing.reason, MfaReason::ChallengeExpiredConsumedOrUnknown);
        assert_eq!(missing.user_id, None);
        assert!(matches!(missing.result, Err(ApplicationError::MfaRejected)));
    }
}

#[test]
fn inactive_and_changed_credentials_are_identified_without_changing_public_error() {
    for reactivate in [false, true] {
        let users = Arc::new(MemoryUsers::default());
        let service = service(users.clone(), Arc::new(MemorySessions::default()));
        let owner = service
            .bootstrap_owner("owner@example.com", "correct horse battery")
            .unwrap();
        let challenge = service
            .start_login("owner@example.com", "correct horse battery")
            .unwrap();
        users.deactivate(owner.principal.id);
        if reactivate {
            users.set_access(owner.principal.id, Role::Owner, true);
        }
        let attempt = service.complete_totp_observed(&challenge.challenge_token, "123456");
        assert_eq!(attempt.user_id, Some(owner.principal.id));
        assert_eq!(
            attempt.reason,
            if reactivate {
                MfaReason::CredentialsChanged
            } else {
                MfaReason::AccountInactive
            }
        );
        assert!(matches!(attempt.result, Err(ApplicationError::MfaRejected)));
    }
}

#[test]
fn recovery_observes_success_and_ambiguous_invalid_or_used_code() {
    let service = service(
        Arc::new(MemoryUsers::default()),
        Arc::new(MemorySessions::default()),
    );
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    for expected in [MfaReason::Accepted, MfaReason::RecoveryInvalidOrUsed] {
        let challenge = service
            .start_login("owner@example.com", "correct horse battery")
            .unwrap();
        let attempt = service
            .complete_recovery_observed(&challenge.challenge_token, &owner.recovery_codes[0]);
        assert_eq!(attempt.reason, expected);
        assert_eq!(attempt.user_id, Some(owner.principal.id));
        assert_eq!(attempt.result.is_ok(), expected == MfaReason::Accepted);
    }
}
