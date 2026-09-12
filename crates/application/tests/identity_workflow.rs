mod identity_support;

use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use application::identity::{EnrollmentResult, IdentityService, SessionStore};
use application::ApplicationError;
use domain::crypto::{TotpEnrollment, TotpProvider, TotpVerification};
use domain::identity::{Permission, Role};
use domain::DomainError;
use identity_support::invalid_email::guarded_service;
use identity_support::{service, service_with_totp, FakeTotp, MemorySessions, MemoryUsers};

#[test]
fn owner_bootstrap_login_totp_authorization_and_logout_form_one_flow() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let service = service(users, sessions.clone());
    let enrollment = service
        .bootstrap_owner(" OWNER@Example.COM ", "correct horse battery")
        .unwrap();
    assert_eq!(enrollment.principal.email, "owner@example.com");
    assert_eq!(enrollment.principal.role, Role::Owner);
    assert_eq!(enrollment.recovery_codes.len(), 8);

    let challenge = service
        .start_login("owner@example.com", "correct horse battery")
        .unwrap();
    let session = service
        .complete_totp(&challenge.challenge_token, "123456")
        .unwrap();
    assert_eq!(
        service
            .authorize(&session.access_token, Permission::CreateUser)
            .unwrap(),
        enrollment.principal
    );

    service.logout(&session.access_token).unwrap();
    assert!(matches!(
        service.authenticate(&session.access_token),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn recovery_codes_are_single_use_and_non_owner_creation_is_denied() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let service = service(users, sessions.clone());
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let owner_token = login_with_recovery(&service, &owner, "correct horse battery");
    let paralegal = service
        .create_user(
            &owner_token,
            "helper@example.com",
            "another safe password",
            Role::Paralegal,
        )
        .unwrap();
    let challenge = service
        .start_login("helper@example.com", "another safe password")
        .unwrap();
    let paralegal_session = service
        .complete_recovery(&challenge.challenge_token, &paralegal.recovery_codes[0])
        .unwrap();

    let second_challenge = service
        .start_login("helper@example.com", "another safe password")
        .unwrap();
    assert!(matches!(
        service.complete_recovery(
            &second_challenge.challenge_token,
            &paralegal.recovery_codes[0]
        ),
        Err(ApplicationError::MfaRejected)
    ));
    assert_eq!(
        sessions
            .take_challenge(&second_challenge.challenge_token)
            .unwrap(),
        None
    );
    assert!(matches!(
        service.create_user(
            &paralegal_session.access_token,
            "forbidden@example.com",
            "another safe password",
            Role::Client,
        ),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn repeated_bad_passwords_lock_the_login_window() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let service = service(users, sessions);
    service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();

    for _ in 0..5 {
        assert!(matches!(
            service.start_login("owner@example.com", "wrong password here"),
            Err(ApplicationError::InvalidCredentials)
        ));
    }
    assert!(matches!(
        service.start_login("owner@example.com", "correct horse battery"),
        Err(ApplicationError::AccountLocked)
    ));
}

struct PausedTotp {
    entered: mpsc::Sender<()>,
    resume: Mutex<mpsc::Receiver<()>>,
}

impl TotpProvider for PausedTotp {
    fn enroll(&self, email: &str) -> Result<TotpEnrollment, DomainError> {
        FakeTotp.enroll(email)
    }

    fn verify(&self, secret: &[u8], code: &str, at: u64) -> Result<TotpVerification, DomainError> {
        self.entered.send(()).unwrap();
        self.resume
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        FakeTotp.verify(secret, code, at)
    }

    fn current_code(&self, secret: &[u8], at: u64) -> Result<String, DomainError> {
        FakeTotp.current_code(secret, at)
    }
}

fn assert_shared_challenge_is_consumed_once(totp_code: &str, totp_should_succeed: bool) {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let service = service_with_totp(
        users,
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

    let (totp, recovery) = std::thread::scope(|scope| {
        let attempt = scope.spawn(|| service.complete_totp(&challenge.challenge_token, totp_code));
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let recovery =
            service.complete_recovery(&challenge.challenge_token, &owner.recovery_codes[0]);
        resume_tx.send(()).unwrap();
        (attempt.join().unwrap(), recovery)
    });

    assert!(matches!(recovery, Err(ApplicationError::MfaRejected)));
    assert_eq!(totp.is_ok(), totp_should_succeed);
    assert_eq!(
        sessions.issued_session_count(),
        usize::from(totp_should_succeed)
    );

    // Losing the challenge must not consume the competing recovery credential.
    let retry = service
        .start_login("owner@example.com", "correct horse battery")
        .unwrap();
    assert!(service
        .complete_recovery(&retry.challenge_token, &owner.recovery_codes[0])
        .is_ok());
}

#[test]
fn concurrent_totp_and_recovery_cannot_issue_two_sessions_for_one_challenge() {
    assert_shared_challenge_is_consumed_once("123456", true);
}

#[test]
fn concurrent_recovery_cannot_reuse_a_challenge_claimed_by_a_rejected_totp() {
    assert_shared_challenge_is_consumed_once("000000", false);
}

#[test]
fn create_user_rejects_an_owner_after_role_downgrade() {
    let users = Arc::new(MemoryUsers::default());
    let service = service(users.clone(), Arc::new(MemorySessions::default()));
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let owner_token = login_with_recovery(&service, &owner, "correct horse battery");
    users.set_role(owner.principal.id, Role::Paralegal);

    assert!(matches!(
        service.create_user(
            &owner_token,
            "forbidden@example.com",
            "another safe password",
            Role::Client
        ),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn create_user_rejects_an_inactive_owner() {
    let users = Arc::new(MemoryUsers::default());
    let service = service(users.clone(), Arc::new(MemorySessions::default()));
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let owner_token = login_with_recovery(&service, &owner, "correct horse battery");
    users.deactivate(owner.principal.id);

    assert!(matches!(
        service.create_user(
            &owner_token,
            "forbidden@example.com",
            "another safe password",
            Role::Client
        ),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn create_user_requires_a_live_session() {
    let service = service(
        Arc::new(MemoryUsers::default()),
        Arc::new(MemorySessions::default()),
    );
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let owner_token = login_with_recovery(&service, &owner, "correct horse battery");
    service.logout(&owner_token).unwrap();

    for token in ["", "not-a-session", &owner_token] {
        assert!(matches!(
            service.create_user(
                token,
                "forbidden@example.com",
                "another safe password",
                Role::Client
            ),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

fn login_with_recovery(
    service: &IdentityService,
    enrollment: &EnrollmentResult,
    password: &str,
) -> String {
    let challenge = service
        .start_login(&enrollment.principal.email, password)
        .unwrap();
    service
        .complete_recovery(&challenge.challenge_token, &enrollment.recovery_codes[0])
        .unwrap()
        .access_token
}

#[test]
fn bootstrap_rejects_control_bytes_before_enrollment() {
    let service = guarded_service(
        Arc::new(MemoryUsers::default()),
        Arc::new(MemorySessions::default()),
    );
    for email in [
        "own\0er@example.com",
        "owner@\x7fexample.com",
        "\nowner@example.com",
        "owner@example.com\t",
    ] {
        assert!(matches!(
            service.bootstrap_owner(email, "correct horse battery"),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
}

#[test]
fn login_and_user_creation_reject_control_bytes_before_email_io() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let enrollment_service = service(users.clone(), sessions.clone());
    let owner = enrollment_service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let owner_token = login_with_recovery(&enrollment_service, &owner, "correct horse battery");
    let service = guarded_service(users, sessions);

    for email in [
        "own\0er@example.com",
        "owner@\x7fexample.com",
        "\nowner@example.com",
        "owner@example.com\t",
    ] {
        assert!(matches!(
            service.start_login(email, "correct horse battery"),
            Err(ApplicationError::InvalidInput(_))
        ));
        assert!(matches!(
            service.create_user(&owner_token, email, "correct horse battery", Role::Client),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
}
