use super::{
    identity_support::{service, service_with_totp, MemorySessions, MemoryUsers},
    PausedTotp,
};
use application::ApplicationError;
use domain::identity::Role;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[test]
fn old_sessions_do_not_revive_after_deactivation_and_reactivation() {
    let users = Arc::new(MemoryUsers::default());
    let service = service(users.clone(), Arc::new(MemorySessions::default()));
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let mut tokens = Vec::new();
    for code in &owner.recovery_codes[..2] {
        let challenge = service
            .start_login("owner@example.com", "correct horse battery")
            .unwrap();
        tokens.push(
            service
                .complete_recovery(&challenge.challenge_token, code)
                .unwrap()
                .access_token,
        );
    }
    users.set_access(owner.principal.id, Role::Owner, false);
    users.set_access(owner.principal.id, Role::Owner, true);
    for token in tokens {
        assert!(matches!(
            service.authenticate(&token),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn old_password_challenge_cannot_take_the_generation_of_a_reactivated_account() {
    let users = Arc::new(MemoryUsers::default());
    let service = service(users.clone(), Arc::new(MemorySessions::default()));
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let challenge = service
        .start_login("owner@example.com", "correct horse battery")
        .unwrap();
    users.set_access(owner.principal.id, Role::Owner, false);
    users.set_access(owner.principal.id, Role::Owner, true);
    assert!(matches!(
        service.complete_recovery(&challenge.challenge_token, &owner.recovery_codes[0]),
        Err(ApplicationError::MfaRejected)
    ));
}

#[test]
fn role_change_during_totp_verification_prevents_session_issuance() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let service = service_with_totp(
        users.clone(),
        sessions.clone(),
        Arc::new(PausedTotp {
            entered: entered_tx,
            resume: Mutex::new(resume_rx),
        }),
    );
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let challenge = service
        .start_login("owner@example.com", "correct horse battery")
        .unwrap();
    let result = std::thread::scope(|scope| {
        let attempt = scope.spawn(|| service.complete_totp(&challenge.challenge_token, "123456"));
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        users.set_access(owner.principal.id, Role::Paralegal, true);
        resume_tx.send(()).unwrap();
        attempt.join().unwrap()
    });
    assert!(matches!(result, Err(ApplicationError::MfaRejected)));
    assert_eq!(sessions.issued_session_count(), 0);
}

#[test]
fn consuming_another_recovery_code_does_not_revoke_existing_generation_sessions() {
    let service = service(
        Arc::new(MemoryUsers::default()),
        Arc::new(MemorySessions::default()),
    );
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let first = super::login_with_recovery(&service, &owner, "correct horse battery");
    let challenge = service
        .start_login("owner@example.com", "correct horse battery")
        .unwrap();
    service
        .complete_recovery(&challenge.challenge_token, &owner.recovery_codes[1])
        .unwrap();
    assert_eq!(service.authenticate(&first).unwrap(), owner.principal);
}
