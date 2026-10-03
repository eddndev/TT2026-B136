//! Identity application service.

use std::sync::{Arc, Mutex};

use domain::audit::AuditLog;
use domain::clock::Clock;
use domain::crypto::{
    PasswordHasher, PasswordVerification, RecoveryCodeGenerator, RecoveryCodeSet, TotpProvider,
    RECOVERY_CODE_COUNT,
};
use domain::identity::{Permission, Role, UserId};

use super::validation::{normalize_email, validate_password};
use super::{
    certificate_login::CertificateLoginPorts, EnrollmentResult, LoginChallenge,
    LoginChallengeIdentity, Principal, SecretProtector, SessionPolicy, SessionStore, UserRecord,
    UserRepository,
};
use crate::ApplicationError;

mod certificate;
mod certificate_authority;
mod issuance;
mod mfa;
mod session;

const CHALLENGE_TTL_SECONDS: u64 = 300;
const FAILURE_WINDOW_SECONDS: u64 = 900;
const TOTP_REPLAY_TTL_SECONDS: u64 = 90;
const MAX_PASSWORD_FAILURES: u32 = 5;

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
    session_policy: SessionPolicy,
    certificate_login: Option<CertificateLoginPorts>,
    audit_log: Mutex<Box<dyn AuditLog + Send + Sync>>,
}

impl IdentityService {
    pub fn new(ports: IdentityPorts) -> Self {
        Self::with_session_policy(ports, SessionPolicy::default())
    }

    pub fn with_session_policy(ports: IdentityPorts, session_policy: SessionPolicy) -> Self {
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
            session_policy,
            certificate_login: None,
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
        if !self
            .ports
            .users
            .insert_initial_owner(record, self.ports.clock.now())?
        {
            return Err(ApplicationError::BootstrapClosed);
        }
        Ok(enrollment)
    }

    pub fn create_user(
        &self,
        access_token: &str,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<EnrollmentResult, ApplicationError> {
        let actor = self.authorize(access_token, Permission::CreateUser)?;
        let email = normalize_email(email)?;
        validate_password(password)?;
        let (record, enrollment) = self.enroll(&email, password, role)?;
        let current = self.authorize(access_token, Permission::CreateUser)?;
        if current != actor {
            return Err(ApplicationError::InvalidSession);
        }
        self.ports
            .users
            .insert(record, current.id, self.ports.clock.now())?;
        Ok(enrollment)
    }

    pub fn start_login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<LoginChallenge, ApplicationError> {
        let email = normalize_email(email)?;
        if self
            .ports
            .sessions
            .failed_password_attempts(&email, FAILURE_WINDOW_SECONDS)?
            >= MAX_PASSWORD_FAILURES
        {
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
        let challenge_token = self.ports.sessions.create_challenge(
            &LoginChallengeIdentity {
                user_id: user.id,
                auth_generation: user.auth_generation,
            },
            CHALLENGE_TTL_SECONDS,
        )?;
        if let Err(error) = self.audit(&email, "identity.password_accepted", &user.id.to_string()) {
            self.ports
                .sessions
                .take_challenge(&challenge_token)
                .map_err(|_| {
                    ApplicationError::Port("challenge cleanup failed after audit failure".into())
                })?;
            return Err(error);
        }
        Ok(LoginChallenge {
            challenge_token,
            expires_in_seconds: CHALLENGE_TTL_SECONDS,
        })
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
            auth_generation: 0,
        };
        let result = EnrollmentResult {
            principal,
            totp_secret_base32: totp.secret_base32,
            otpauth_uri: totp.otpauth_uri,
            recovery_codes: plain_codes,
        };
        Ok((record, result))
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
