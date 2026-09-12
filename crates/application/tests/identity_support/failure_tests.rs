use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::Duration;

use application::identity::IdentityService;
use application::ApplicationError;
use domain::audit::{AuditLog, ChainedEvent};
use domain::clock::OffsetDateTime;
use domain::DomainError;

use super::{ports, service_with_totp, MemoryAudit, MemorySessions, MemoryUsers};

struct SwitchableAudit {
    fail: Arc<AtomicBool>,
    log: MemoryAudit,
}

impl AuditLog for SwitchableAudit {
    fn append(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
        at: OffsetDateTime,
    ) -> Result<ChainedEvent, DomainError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(DomainError::AuditStorageFailure("audit unavailable".into()));
        }
        self.log.append(actor, action, resource, at)
    }

    fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError> {
        self.log.load_all()
    }
}

fn service(
    users: Arc<MemoryUsers>,
    sessions: Arc<MemorySessions>,
    fail: Arc<AtomicBool>,
) -> IdentityService {
    let mut ports = ports(users, sessions);
    ports.audit_log = Box::new(SwitchableAudit {
        fail,
        log: MemoryAudit::default(),
    });
    IdentityService::new(ports)
}

#[test]
fn audit_failure_discards_a_new_password_verified_challenge() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let service = service(users, sessions.clone(), Arc::new(AtomicBool::new(true)));
    service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();

    let error = service
        .start_login("owner@example.com", "correct horse battery")
        .err()
        .unwrap();
    assert!(matches!(
        error,
        ApplicationError::Domain(DomainError::AuditStorageFailure(_))
    ));
    assert_eq!(sessions.active_challenge_count(), 0);
    assert_eq!(sessions.active_session_count(), 0);
    assert!(!error.to_string().contains("challenge-"));
}

#[test]
fn audit_failure_discards_sessions_created_by_either_second_factor() {
    for recovery in [false, true] {
        let users = Arc::new(MemoryUsers::default());
        let sessions = Arc::new(MemorySessions::default());
        let fail = Arc::new(AtomicBool::new(false));
        let service = service(users, sessions.clone(), fail.clone());
        let owner = service
            .bootstrap_owner("owner@example.com", "correct horse battery")
            .unwrap();
        let challenge = service
            .start_login("owner@example.com", "correct horse battery")
            .unwrap();
        fail.store(true, Ordering::SeqCst);

        let result = if recovery {
            service.complete_recovery(&challenge.challenge_token, &owner.recovery_codes[0])
        } else {
            service.complete_totp(&challenge.challenge_token, "123456")
        };
        let error = result.err().unwrap();
        assert!(matches!(
            error,
            ApplicationError::Domain(DomainError::AuditStorageFailure(_))
        ));
        assert_eq!(sessions.active_session_count(), 0);
        assert_eq!(sessions.active_challenge_count(), 0);
        assert_eq!(sessions.issued_session_count(), 1);
        assert!(!error.to_string().contains("session-"));
    }
}

#[test]
fn logout_audit_failure_keeps_the_session_revoked() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let fail = Arc::new(AtomicBool::new(false));
    let service = service(users, sessions.clone(), fail.clone());
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let token = crate::login_with_recovery(&service, &owner, "correct horse battery");
    fail.store(true, Ordering::SeqCst);

    assert!(service.logout(&token).is_err());
    assert_eq!(sessions.active_session_count(), 0);
    assert!(matches!(
        service.authenticate(&token),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn deactivation_during_totp_verification_prevents_session_creation() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let service = service_with_totp(
        users.clone(),
        sessions.clone(),
        Arc::new(crate::PausedTotp {
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
        users.deactivate(owner.principal.id);
        resume_tx.send(()).unwrap();
        attempt.join().unwrap()
    });
    assert!(matches!(result, Err(ApplicationError::MfaRejected)));
    assert_eq!(sessions.issued_session_count(), 0);
    assert_eq!(sessions.active_challenge_count(), 0);
}

#[test]
fn failed_challenge_compensation_does_not_expose_its_token_in_the_error() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    sessions
        .fail_challenge_cleanup
        .store(true, Ordering::SeqCst);
    let service = service(users, sessions.clone(), Arc::new(AtomicBool::new(true)));
    service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let error = service
        .start_login("owner@example.com", "correct horse battery")
        .err()
        .unwrap();
    assert!(matches!(error, ApplicationError::Port(_)));
    assert!(!error.to_string().contains("challenge-"));
    // A failed Redis cleanup cannot be represented as a successful rollback.
    assert_eq!(sessions.active_challenge_count(), 1);
}

#[test]
fn failed_session_compensation_does_not_expose_its_token_in_the_error() {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    let fail = Arc::new(AtomicBool::new(false));
    let service = service(users, sessions.clone(), fail.clone());
    service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let challenge = service
        .start_login("owner@example.com", "correct horse battery")
        .unwrap();
    fail.store(true, Ordering::SeqCst);
    sessions.fail_session_cleanup.store(true, Ordering::SeqCst);
    let error = service
        .complete_totp(&challenge.challenge_token, "123456")
        .err()
        .unwrap();
    assert!(matches!(error, ApplicationError::Port(_)));
    assert!(!error.to_string().contains("session-"));
    assert_eq!(sessions.active_session_count(), 1);
    assert_eq!(sessions.active_challenge_count(), 0);
}
