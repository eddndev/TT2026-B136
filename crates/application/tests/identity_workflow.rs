use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use application::identity::{
    IdentityPorts, IdentityService, Principal, SecretProtector, SessionStore, UserRecord,
    UserRepository,
};
use application::ApplicationError;
use domain::audit::{AuditEvent, AuditLog, ChainedEvent};
use domain::clock::{Clock, OffsetDateTime};
use domain::crypto::{
    PasswordHasher, PasswordVerification, RecoveryCodeGenerator, Sha256Digest, TotpEnrollment,
    TotpProvider, TotpVerification,
};
use domain::identity::{Permission, Role, UserId};
use domain::DomainError;
use zeroize::Zeroizing;

#[derive(Default)]
struct MemoryUsers(Mutex<HashMap<UserId, UserRecord>>);

impl UserRepository for MemoryUsers {
    fn has_users(&self) -> Result<bool, ApplicationError> {
        Ok(!self.0.lock().unwrap().is_empty())
    }

    fn insert_initial_owner(&self, user: UserRecord) -> Result<bool, ApplicationError> {
        let mut users = self.0.lock().unwrap();
        if !users.is_empty() {
            return Ok(false);
        }
        users.insert(user.id, user);
        Ok(true)
    }

    fn insert(&self, user: UserRecord) -> Result<(), ApplicationError> {
        let mut users = self.0.lock().unwrap();
        if users.values().any(|stored| stored.email == user.email) {
            return Err(ApplicationError::UserAlreadyExists);
        }
        users.insert(user.id, user);
        Ok(())
    }

    fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, ApplicationError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .values()
            .find(|user| user.email == email)
            .cloned())
    }

    fn find_by_id(&self, id: UserId) -> Result<Option<UserRecord>, ApplicationError> {
        Ok(self.0.lock().unwrap().get(&id).cloned())
    }

    fn replace_recovery_codes(
        &self,
        id: UserId,
        expected_revision: u64,
        codes: domain::crypto::RecoveryCodeSet,
    ) -> Result<(), ApplicationError> {
        let mut users = self.0.lock().unwrap();
        let user = users.get_mut(&id).ok_or(ApplicationError::UserNotFound)?;
        if user.revision != expected_revision {
            return Err(ApplicationError::ConcurrentModification);
        }
        user.recovery_codes = codes;
        user.revision += 1;
        Ok(())
    }
}

#[derive(Default)]
struct MemorySessions {
    challenges: Mutex<HashMap<String, UserId>>,
    sessions: Mutex<HashMap<String, Principal>>,
    failures: Mutex<HashMap<String, u32>>,
    used_totp: Mutex<Vec<(UserId, String)>>,
}

impl SessionStore for MemorySessions {
    fn create_challenge(&self, user_id: UserId, _ttl: u64) -> Result<String, ApplicationError> {
        let token = format!("challenge-{user_id}");
        self.challenges
            .lock()
            .unwrap()
            .insert(token.clone(), user_id);
        Ok(token)
    }

    fn resolve_challenge(&self, token: &str) -> Result<Option<UserId>, ApplicationError> {
        Ok(self.challenges.lock().unwrap().get(token).copied())
    }

    fn consume_challenge(&self, token: &str) -> Result<(), ApplicationError> {
        self.challenges.lock().unwrap().remove(token);
        Ok(())
    }

    fn create_session(&self, principal: &Principal, _ttl: u64) -> Result<String, ApplicationError> {
        let token = format!("session-{}", principal.id);
        self.sessions
            .lock()
            .unwrap()
            .insert(token.clone(), principal.clone());
        Ok(token)
    }

    fn find_session(&self, token: &str) -> Result<Option<Principal>, ApplicationError> {
        Ok(self.sessions.lock().unwrap().get(token).cloned())
    }

    fn revoke_session(&self, token: &str) -> Result<(), ApplicationError> {
        self.sessions.lock().unwrap().remove(token);
        Ok(())
    }

    fn failed_password_attempts(&self, email: &str) -> Result<u32, ApplicationError> {
        Ok(*self.failures.lock().unwrap().get(email).unwrap_or(&0))
    }

    fn record_password_failure(&self, email: &str, _ttl: u64) -> Result<u32, ApplicationError> {
        let mut failures = self.failures.lock().unwrap();
        let count = failures.entry(email.to_string()).or_default();
        *count += 1;
        Ok(*count)
    }

    fn clear_password_failures(&self, email: &str) -> Result<(), ApplicationError> {
        self.failures.lock().unwrap().remove(email);
        Ok(())
    }

    fn claim_totp(
        &self,
        user_id: UserId,
        code_fingerprint: &str,
        _ttl: u64,
    ) -> Result<bool, ApplicationError> {
        let key = (user_id, code_fingerprint.to_string());
        let mut used = self.used_totp.lock().unwrap();
        if used.contains(&key) {
            return Ok(false);
        }
        used.push(key);
        Ok(true)
    }
}

struct FakeHasher;

impl PasswordHasher for FakeHasher {
    fn hash(&self, value: &str) -> Result<String, DomainError> {
        Ok(format!("fake:{value}"))
    }

    fn verify(&self, value: &str, stored: &str) -> Result<PasswordVerification, DomainError> {
        Ok(if stored == format!("fake:{value}") {
            PasswordVerification::Match
        } else {
            PasswordVerification::Mismatch
        })
    }
}

struct FakeTotp;

impl TotpProvider for FakeTotp {
    fn enroll(&self, email: &str) -> Result<TotpEnrollment, DomainError> {
        Ok(TotpEnrollment {
            secret: Zeroizing::new(vec![7; 20]),
            secret_base32: Zeroizing::new("A7A7A7A7".to_string()),
            otpauth_uri: Zeroizing::new(format!("otpauth://totp/app:{email}")),
        })
    }

    fn verify(&self, secret: &[u8], code: &str, _at: u64) -> Result<TotpVerification, DomainError> {
        Ok(if secret == [7; 20] && code == "123456" {
            TotpVerification::Accepted
        } else {
            TotpVerification::Rejected
        })
    }

    fn current_code(&self, _secret: &[u8], _at: u64) -> Result<String, DomainError> {
        Ok("123456".to_string())
    }
}

struct FakeRecovery;

impl RecoveryCodeGenerator for FakeRecovery {
    fn generate(&self, count: usize) -> Result<Vec<Zeroizing<String>>, DomainError> {
        Ok((0..count)
            .map(|index| Zeroizing::new(format!("RECOVERY-{index}")))
            .collect())
    }
}

struct FakeProtector;

impl SecretProtector for FakeProtector {
    fn protect(&self, _id: UserId, secret: &[u8]) -> Result<Vec<u8>, ApplicationError> {
        Ok(secret.to_vec())
    }

    fn expose(
        &self,
        _id: UserId,
        protected: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
        Ok(Zeroizing::new(protected.to_vec()))
    }
}

struct FrozenClock;

impl Clock for FrozenClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap()
    }
}

#[derive(Default)]
struct MemoryAudit(Vec<ChainedEvent>);

impl AuditLog for MemoryAudit {
    fn append(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
        timestamp: OffsetDateTime,
    ) -> Result<ChainedEvent, DomainError> {
        let event = ChainedEvent {
            event: AuditEvent::new(self.0.len() as u64, timestamp, actor, action, resource),
            chain: Sha256Digest::from_array([0; 32]),
        };
        self.0.push(event.clone());
        Ok(event)
    }

    fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError> {
        Ok(self.0.clone())
    }
}

fn service(users: Arc<MemoryUsers>, sessions: Arc<MemorySessions>) -> IdentityService {
    IdentityService::new(IdentityPorts {
        users,
        sessions,
        passwords: Arc::new(FakeHasher),
        totp: Arc::new(FakeTotp),
        recovery: Arc::new(FakeRecovery),
        secrets: Arc::new(FakeProtector),
        clock: Arc::new(FrozenClock),
        audit_log: Box::new(MemoryAudit::default()),
    })
}

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
    let paralegal = service
        .create_user(
            &owner.principal,
            "helper@example.com",
            "another safe password",
            Role::Paralegal,
        )
        .unwrap();
    let challenge = service
        .start_login("helper@example.com", "another safe password")
        .unwrap();
    service
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
            .resolve_challenge(&second_challenge.challenge_token)
            .unwrap(),
        None
    );
    assert!(matches!(
        service.create_user(
            &paralegal.principal,
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
