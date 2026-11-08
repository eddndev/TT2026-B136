use std::sync::Arc;

use application::identity::{IdentityService, IdentityWorkflow, UserRepository};
use application::ApplicationError;
use domain::identity::Role;
use domain::DomainError;

use crate::identity_support::invalid_email::guarded_service;
use crate::identity_support::{service, MemorySessions, MemoryUsers};

const PASSWORD: &str = "correct horse battery";

fn totp_session(service: &IdentityService, email: &str) -> application::identity::SessionResult {
    let challenge = service.start_login(email, PASSWORD).unwrap();
    service
        .complete_totp(&challenge.challenge_token, "123456")
        .unwrap()
}

fn invalid_name(result: Result<impl Sized, ApplicationError>) -> bool {
    matches!(
        result,
        Err(ApplicationError::Domain(DomainError::InvalidDisplayName(_)))
    )
}

#[test]
fn bootstrap_stores_the_trimmed_owner_name() {
    let users = Arc::new(MemoryUsers::default());
    let service = service(users.clone(), Arc::new(MemorySessions::default()));
    let owner = service
        .bootstrap_owner("owner@example.com", "  Ana  Garc\u{ed}a ", PASSWORD)
        .unwrap();
    assert_eq!(owner.display_name.as_str(), "Ana  Garc\u{ed}a");
    let stored = users.find_by_id(owner.principal.id).unwrap().unwrap();
    assert_eq!(stored.display_name.as_str(), "Ana  Garc\u{ed}a");
    assert_eq!(stored.revision, 0);
    assert_eq!(stored.auth_generation, 0);
}

#[test]
fn invalid_names_fail_before_hashing_or_persistence() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let guarded = guarded_service(users.clone(), sessions.clone());
    let long = "a".repeat(121);
    let rejected = [
        "",
        "   ",
        "Ana\n",
        "\u{202e}Ana",
        "Ana\u{200b}",
        long.as_str(),
    ];
    for name in rejected {
        assert!(
            invalid_name(guarded.bootstrap_owner("owner@example.com", name, PASSWORD)),
            "{name:?}"
        );
    }
    let enrolling = service(users.clone(), sessions.clone());
    enrolling
        .bootstrap_owner("owner@example.com", "Owner", PASSWORD)
        .unwrap();
    let token = totp_session(&enrolling, "owner@example.com").access_token;
    for name in rejected {
        assert!(
            invalid_name(guarded.create_user(
                &token,
                "member@example.com",
                name,
                PASSWORD,
                Role::Litigator
            )),
            "{name:?}"
        );
    }
    assert!(matches!(
        guarded.create_user("missing", "member@example.com", "", PASSWORD, Role::Client),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        guarded.create_user(&token, "bad-email", "", PASSWORD, Role::Client),
        Err(ApplicationError::InvalidInput(_))
    ));
}

#[test]
fn the_owner_names_litigator_and_client_accounts() {
    let users = Arc::new(MemoryUsers::default());
    let service = service(users.clone(), Arc::new(MemorySessions::default()));
    service
        .bootstrap_owner("owner@example.com", "Owner", PASSWORD)
        .unwrap();
    let token = totp_session(&service, "owner@example.com").access_token;
    for (email, name, role) in [
        ("litigator@example.com", " Luis P\u{e9}rez", Role::Litigator),
        ("client@example.com", "Cliente Uno ", Role::Client),
    ] {
        let created = service
            .create_user(&token, email, name, PASSWORD, role)
            .unwrap();
        assert_eq!(created.display_name.as_str(), name.trim());
        let stored = users.find_by_id(created.principal.id).unwrap().unwrap();
        assert_eq!(stored.display_name, created.display_name);
        assert_eq!(stored.role, role);
    }
}

#[test]
fn sessions_carry_the_name_without_changing_the_principal() {
    let users = Arc::new(MemoryUsers::default());
    let service = service(users, Arc::new(MemorySessions::default()));
    let owner = service
        .bootstrap_owner("owner@example.com", "Hatziel Ortega", PASSWORD)
        .unwrap();
    let session = totp_session(&service, "owner@example.com");
    assert_eq!(session.session.display_name.as_str(), "Hatziel Ortega");
    assert_eq!(session.principal, owner.principal);
    let token = session.access_token.as_str();
    assert_eq!(
        service.session_status(token).unwrap().display_name.as_str(),
        "Hatziel Ortega"
    );
    assert_eq!(
        service
            .record_activity(token)
            .unwrap()
            .display_name
            .as_str(),
        "Hatziel Ortega"
    );
    let workflow: &dyn IdentityWorkflow = &service;
    let current = workflow.current_account(token).unwrap();
    assert_eq!(current.display_name.as_str(), "Hatziel Ortega");
    assert_eq!(current.principal, workflow.authenticate(token).unwrap());
    workflow.logout(token).unwrap();
    assert!(matches!(
        workflow.current_account(token),
        Err(ApplicationError::InvalidSession)
    ));
}
