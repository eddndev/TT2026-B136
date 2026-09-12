//! Outbound persistence and secret-protection ports for identity use cases.

use domain::crypto::RecoveryCodeSet;
use domain::identity::{Permission, Role, UserId};
use zeroize::Zeroizing;

use super::{EnrollmentResult, LoginChallenge, Principal, SessionResult, UserRecord};
use crate::ApplicationError;

/// Inbound boundary consumed by delivery adapters.
pub trait IdentityWorkflow: Send + Sync {
    fn bootstrap_owner(
        &self,
        email: &str,
        password: &str,
    ) -> Result<EnrollmentResult, ApplicationError>;
    fn create_user(
        &self,
        access_token: &str,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<EnrollmentResult, ApplicationError>;
    fn start_login(&self, email: &str, password: &str) -> Result<LoginChallenge, ApplicationError>;
    fn complete_totp(
        &self,
        challenge_token: &str,
        code: &str,
    ) -> Result<SessionResult, ApplicationError>;
    fn complete_recovery(
        &self,
        challenge_token: &str,
        code: &str,
    ) -> Result<SessionResult, ApplicationError>;
    fn authenticate(&self, access_token: &str) -> Result<Principal, ApplicationError>;
    fn authorize(
        &self,
        access_token: &str,
        permission: Permission,
    ) -> Result<Principal, ApplicationError>;
    fn logout(&self, access_token: &str) -> Result<(), ApplicationError>;
}

/// Durable user persistence.
pub trait UserRepository: Send + Sync {
    fn has_users(&self) -> Result<bool, ApplicationError>;
    /// Atomically inserts the first owner, or returns false if any user exists.
    fn insert_initial_owner(&self, user: UserRecord) -> Result<bool, ApplicationError>;
    fn insert(&self, user: UserRecord) -> Result<(), ApplicationError>;
    fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, ApplicationError>;
    fn find_by_id(&self, id: UserId) -> Result<Option<UserRecord>, ApplicationError>;
    fn replace_recovery_codes(
        &self,
        id: UserId,
        expected_revision: u64,
        codes: RecoveryCodeSet,
    ) -> Result<(), ApplicationError>;
}

/// Ephemeral challenges, sessions, throttling counters, and TOTP replay keys.
pub trait SessionStore: Send + Sync {
    fn create_challenge(&self, user_id: UserId, ttl: u64) -> Result<String, ApplicationError>;
    /// Atomically removes and returns a live challenge before any MFA attempt.
    /// Missing, expired, and already claimed challenges all return None.
    fn take_challenge(&self, token: &str) -> Result<Option<UserId>, ApplicationError>;
    fn create_session(&self, principal: &Principal, ttl: u64) -> Result<String, ApplicationError>;
    fn find_session(&self, token: &str) -> Result<Option<Principal>, ApplicationError>;
    fn revoke_session(&self, token: &str) -> Result<(), ApplicationError>;
    /// Reads the failure count and repairs missing expiration using the supplied window.
    /// Existing expirations are preserved; absent counters remain absent.
    fn failed_password_attempts(&self, email: &str, ttl: u64) -> Result<u32, ApplicationError>;
    fn record_password_failure(&self, email: &str, ttl: u64) -> Result<u32, ApplicationError>;
    fn clear_password_failures(&self, email: &str) -> Result<(), ApplicationError>;
    /// Claims a valid code once. Implementations persist only a fingerprint.
    fn claim_totp(&self, user_id: UserId, code: &str, ttl: u64) -> Result<bool, ApplicationError>;
}

/// Encrypts TOTP secrets with user-specific authenticated context.
pub trait SecretProtector: Send + Sync {
    fn protect(&self, id: UserId, secret: &[u8]) -> Result<Vec<u8>, ApplicationError>;
    fn expose(&self, id: UserId, protected: &[u8]) -> Result<Zeroizing<Vec<u8>>, ApplicationError>;
}
