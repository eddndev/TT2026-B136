use std::sync::Arc;

use application::ca_access::{CaOperatorAccess, OperatorIdentity};
use application::identity::UserRepository;
use application::ApplicationError;
use domain::identity::{Permission, Role, UserId};
use time::OffsetDateTime;

use crate::identity_support::{service, MemorySessions, MemoryUsers};

#[test]
fn preparation_keeps_office_accounts_unchanged_and_grants_no_office_authority() {
    let users = Arc::new(MemoryUsers::default());
    let identity = service(users.clone(), Arc::new(MemorySessions::default()));
    let owner = identity
        .bootstrap_owner("owner@example.test", "existing owner password")
        .unwrap();
    let (record, enrollment) = identity
        .prepare_ca_operator(" CA@Example.TEST ", "separate ca password")
        .unwrap();
    assert_eq!(record.email, "ca@example.test");
    assert_eq!(record.id, enrollment.principal.id);
    assert_eq!(record.role, Role::Client);
    assert_ne!(record.id, owner.principal.id);
    assert_eq!(enrollment.recovery_codes.len(), 8);
    assert!(!record.role.allows(Permission::CreateUser));
    assert!(users.find_by_id(record.id).unwrap().is_none());
    assert_eq!(
        users
            .find_by_email("owner@example.test")
            .unwrap()
            .unwrap()
            .role,
        Role::Owner
    );
    assert!(identity
        .prepare_ca_operator("bad-email", "separate ca password")
        .is_err());
    assert!(identity
        .prepare_ca_operator("ca@example.test", "short")
        .is_err());
}

#[test]
fn ca_access_requires_the_configured_identity_after_mfa_and_rejects_an_office_owner() {
    let users = Arc::new(MemoryUsers::default());
    let identity = Arc::new(service(users.clone(), Arc::new(MemorySessions::default())));
    let owner = identity
        .bootstrap_owner("owner@example.test", "existing owner password")
        .unwrap();
    let (record, enrollment) = identity
        .prepare_ca_operator("ca@example.test", "separate ca password")
        .unwrap();
    users
        .insert(
            record.clone(),
            owner.principal.id,
            OffsetDateTime::now_utc(),
        )
        .unwrap();
    let configured = OperatorIdentity::new(record.id, &record.email).unwrap();
    let access = CaOperatorAccess::new(identity.clone(), configured.clone());
    assert!(matches!(
        access.start_login("owner@example.test", "existing owner password"),
        Err(ApplicationError::InvalidCredentials)
    ));
    let challenge = access
        .start_login("ca@example.test", "separate ca password")
        .unwrap();
    let accepted = access
        .complete_recovery_observed(&challenge.challenge_token, &enrollment.recovery_codes[0]);
    assert_eq!(accepted.reason, application::identity::MfaReason::Accepted);
    assert_eq!(accepted.user_id, Some(record.id));
    let session = accepted.result.unwrap();
    assert_eq!(
        access.authenticate(&session.access_token).unwrap(),
        configured
    );
    let wrong = CaOperatorAccess::new(
        identity.clone(),
        OperatorIdentity::new(UserId::new(), &record.email).unwrap(),
    );
    assert!(wrong.authenticate(&session.access_token).is_err());
    let challenge = identity
        .start_login("owner@example.test", "existing owner password")
        .unwrap();
    let rejected =
        access.complete_recovery_observed(&challenge.challenge_token, &owner.recovery_codes[0]);
    assert_eq!(
        rejected.reason,
        application::identity::MfaReason::SessionAdmissionRejected
    );
    assert_eq!(rejected.user_id, Some(owner.principal.id));
    assert!(matches!(
        rejected.result,
        Err(ApplicationError::PermissionDenied)
    ));
    access.logout(&session.access_token).unwrap();
    assert!(access.authenticate(&session.access_token).is_err());
}
