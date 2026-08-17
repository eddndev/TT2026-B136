//! Identity application service.

use std::sync::{Arc, Mutex};

use domain::audit::AuditLog;
use domain::clock::Clock;
use domain::crypto::{
    PasswordHasher, PasswordVerification, RecoveryCodeGenerator, RecoveryCodeOutcome,
    RecoveryCodeSet, TotpProvider, TotpVerification, RECOVERY_CODE_COUNT,
};
use domain::identity::{Permission, Role, UserId};

use super::{
    EnrollmentResult, LoginChallenge, Principal, SecretProtector, SessionResult, SessionStore,
    UserRecord, UserRepository,
};
use crate::ApplicationError;

const CHALLENGE_TTL_SECONDS: u64 = 300;
const SESSION_TTL_SECONDS: u64 = 86_400;
const FAILURE_WINDOW_SECONDS: u64 = 900;
const TOTP_REPLAY_TTL_SECONDS: u64 = 90;
const MAX_PASSWORD_FAILURES: u32 = 5;
const MIN_PASSWORD_BYTES: usize = 12;
const MAX_PASSWORD_BYTES: usize = 1024;

/// Outbound adapters required by [`IdentityService`].
pub struct IdentityPorts {
    pub users: Arc<dyn UserRepository>,
    pub sessions: Arc<dyn SessionStore>,
    pub passwords: Arc<dyn PasswordHasher + Send + Sync>,
    pub totp: Arc<dyn TotpProvider + Send + Sync>,
    pub recovery: Arc<dyn RecoveryCodeGenerator + Send + Sync>,
    pub secrets: Arc<dyn SecretProtector>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub audit_log: Box<dyn AuditLog + Send + Sync>,
}

struct RuntimePorts {
    users: Arc<dyn UserRepository>,
    sessions: Arc<dyn SessionStore>,
    passwords: Arc<dyn PasswordHasher + Send + Sync>,
    totp: Arc<dyn TotpProvider + Send + Sync>,
    recovery: Arc<dyn RecoveryCodeGenerator + Send + Sync>,
    secrets: Arc<dyn SecretProtector>,
    clock: Arc<dyn Clock + Send + Sync>,
}

/// Thread-safe implementation of account and session workflows.
pub struct IdentityService {
    ports: RuntimePorts,
    audit_log: Mutex<Box<dyn AuditLog + Send + Sync>>,
}

impl IdentityService {
    pub fn new(ports: IdentityPorts) -> Self {
        let IdentityPorts {
            users,
            sessions,
            passwords,
            totp,
            recovery,
            secrets,
            clock,
            audit_log,
        } = ports;
        Self {
            ports: RuntimePorts {
                users,
                sessions,
                passwords,
                totp,
                recovery,
                secrets,
                clock,
            },
            audit_log: Mutex::new(audit_log),
        }
    }

    pub fn bootstrap_owner(
        &self,
        email: &str,
        password: &str,
    ) -> Result<EnrollmentResult, ApplicationError> {
        if self.ports.users.has_users()? {
            return Err(ApplicationError::BootstrapClosed);
        }
        let email = normalize_email(email)?;
        validate_password(password)?;
        let (record, enrollment) = self.enroll(&email, password, Role::Owner)?;
        if !self.ports.users.insert_initial_owner(record)? {
            return Err(ApplicationError::BootstrapClosed);
        }
        self.audit("system", "identity.owner_bootstrapped", &email)?;
        Ok(enrollment)
    }

    pub fn create_user(
        &self,
        actor: &Principal,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<EnrollmentResult, ApplicationError> {
        if !actor.role.allows(Permission::CreateUser) {
            return Err(ApplicationError::PermissionDenied);
        }
        let email = normalize_email(email)?;
        validate_password(password)?;
        let (record, enrollment) = self.enroll(&email, password, role)?;
        self.ports.users.insert(record)?;
        self.audit(&actor.email, "identity.user_created", &email)?;
        Ok(enrollment)
    }

    pub fn start_login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<LoginChallenge, ApplicationError> {
        let email = normalize_email(email)?;
        if self.ports.sessions.failed_password_attempts(&email)? >= MAX_PASSWORD_FAILURES {
            return Err(ApplicationError::AccountLocked);
        }
        let Some(user) = self.ports.users.find_by_email(&email)? else {
            let _discarded_hash = self.ports.passwords.hash(password)?;
            self.reject_password(&email)?;
            return Err(ApplicationError::InvalidCredentials);
        };
        if !user.active
            || self.ports.passwords.verify(password, &user.password_hash)?
                != PasswordVerification::Match
        {
            self.reject_password(&email)?;
            return Err(ApplicationError::InvalidCredentials);
        }
        self.ports.sessions.clear_password_failures(&email)?;
        let challenge_token = self
            .ports
            .sessions
            .create_challenge(user.id, CHALLENGE_TTL_SECONDS)?;
        self.audit(&email, "identity.password_accepted", &user.id.to_string())?;
        Ok(LoginChallenge {
            challenge_token,
            expires_in_seconds: CHALLENGE_TTL_SECONDS,
        })
    }

    pub fn complete_totp(
        &self,
        challenge_token: &str,
        code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        let user = self.challenge_user(challenge_token)?;
        let secret = self
            .ports
            .secrets
            .expose(user.id, &user.protected_totp_secret)?;
        let unix = self.ports.clock.now().unix_timestamp().max(0) as u64;
        let accepted = self.ports.totp.verify(&secret, code, unix)? == TotpVerification::Accepted
            && self
                .ports
                .sessions
                .claim_totp(user.id, code, TOTP_REPLAY_TTL_SECONDS)?;
        if !accepted {
            self.ports.sessions.consume_challenge(challenge_token)?;
            return Err(ApplicationError::MfaRejected);
        }
        self.finish_challenge(challenge_token, &user, "identity.totp_accepted")
    }

    pub fn complete_recovery(
        &self,
        challenge_token: &str,
        code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        let mut user = self.challenge_user(challenge_token)?;
        let expected_revision = user.revision;
        if user
            .recovery_codes
            .consume(code, self.ports.passwords.as_ref())?
            != RecoveryCodeOutcome::Accepted
        {
            self.ports.sessions.consume_challenge(challenge_token)?;
            return Err(ApplicationError::MfaRejected);
        }
        self.ports.users.replace_recovery_codes(
            user.id,
            expected_revision,
            user.recovery_codes.clone(),
        )?;
        self.finish_challenge(challenge_token, &user, "identity.recovery_accepted")
    }

    pub fn authenticate(&self, access_token: &str) -> Result<Principal, ApplicationError> {
        let cached = self
            .ports
            .sessions
            .find_session(access_token)?
            .ok_or(ApplicationError::InvalidSession)?;
        let Some(user) = self.ports.users.find_by_id(cached.id)? else {
            self.ports.sessions.revoke_session(access_token)?;
            return Err(ApplicationError::InvalidSession);
        };
        if !user.active {
            self.ports.sessions.revoke_session(access_token)?;
            return Err(ApplicationError::InvalidSession);
        }
        Ok(Principal::from(&user))
    }

    pub fn authorize(
        &self,
        access_token: &str,
        permission: Permission,
    ) -> Result<Principal, ApplicationError> {
        let principal = self.authenticate(access_token)?;
        if !principal.role.allows(permission) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(principal)
    }

    pub fn logout(&self, access_token: &str) -> Result<(), ApplicationError> {
        let principal = self.authenticate(access_token)?;
        self.ports.sessions.revoke_session(access_token)?;
        self.audit(
            &principal.email,
            "identity.session_revoked",
            &principal.id.to_string(),
        )
    }

    fn enroll(
        &self,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<(UserRecord, EnrollmentResult), ApplicationError> {
        let id = UserId::new();
        let password_hash = self.ports.passwords.hash(password)?;
        let totp = self.ports.totp.enroll(email)?;
        let plain_codes = self.ports.recovery.generate(RECOVERY_CODE_COUNT)?;
        let mut hashes = Vec::with_capacity(plain_codes.len());
        for code in &plain_codes {
            hashes.push(self.ports.passwords.hash(code)?);
        }
        let recovery_codes = RecoveryCodeSet::from_hashes(hashes)?;
        let protected_totp_secret = self.ports.secrets.protect(id, &totp.secret)?;
        let principal = Principal {
            id,
            email: email.to_string(),
            role,
        };
        let record = UserRecord {
            id,
            email: email.to_string(),
            password_hash,
            role,
            active: true,
            protected_totp_secret,
            recovery_codes,
            revision: 0,
        };
        let result = EnrollmentResult {
            principal,
            totp_secret_base32: totp.secret_base32,
            otpauth_uri: totp.otpauth_uri,
            recovery_codes: plain_codes,
        };
        Ok((record, result))
    }

    fn challenge_user(&self, token: &str) -> Result<UserRecord, ApplicationError> {
        let id = self
            .ports
            .sessions
            .resolve_challenge(token)?
            .ok_or(ApplicationError::MfaRejected)?;
        self.ports
            .users
            .find_by_id(id)?
            .filter(|user| user.active)
            .ok_or(ApplicationError::MfaRejected)
    }

    fn finish_challenge(
        &self,
        token: &str,
        user: &UserRecord,
        action: &str,
    ) -> Result<SessionResult, ApplicationError> {
        let principal = Principal::from(user);
        let access_token = self
            .ports
            .sessions
            .create_session(&principal, SESSION_TTL_SECONDS)?;
        self.ports.sessions.consume_challenge(token)?;
        self.audit(&principal.email, action, &principal.id.to_string())?;
        Ok(SessionResult {
            access_token,
            expires_in_seconds: SESSION_TTL_SECONDS,
            principal,
        })
    }

    fn reject_password(&self, email: &str) -> Result<(), ApplicationError> {
        self.ports
            .sessions
            .record_password_failure(email, FAILURE_WINDOW_SECONDS)?;
        self.audit(email, "identity.password_rejected", "login")
    }

    fn audit(&self, actor: &str, action: &str, resource: &str) -> Result<(), ApplicationError> {
        self.audit_log
            .lock()
            .map_err(|_| ApplicationError::Port("audit lock poisoned".to_string()))?
            .append(actor, action, resource, self.ports.clock.now())?;
        Ok(())
    }
}

fn normalize_email(value: &str) -> Result<String, ApplicationError> {
    let normalized = value.trim().to_ascii_lowercase();
    let mut parts = normalized.split('@');
    let valid = normalized.is_ascii()
        && normalized.len() <= 254
        && !normalized.contains(char::is_whitespace)
        && parts.next().is_some_and(|part| !part.is_empty())
        && parts.next().is_some_and(|part| !part.is_empty())
        && parts.next().is_none();
    if !valid {
        return Err(ApplicationError::InvalidInput("invalid email".to_string()));
    }
    Ok(normalized)
}

fn validate_password(value: &str) -> Result<(), ApplicationError> {
    if !(MIN_PASSWORD_BYTES..=MAX_PASSWORD_BYTES).contains(&value.len()) {
        return Err(ApplicationError::InvalidInput(format!(
            "password must contain between {MIN_PASSWORD_BYTES} and {MAX_PASSWORD_BYTES} bytes"
        )));
    }
    Ok(())
}
