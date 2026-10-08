//! Outbound persistence and secret-protection ports for identity use cases.

use domain::clock::OffsetDateTime;
use domain::crypto::RecoveryCodeSet;
use domain::identity::{Permission, Role, UserId};
use zeroize::Zeroizing;

use super::{
    certificate_login::{CertificateMfaChallenge, CertificateSessionProvenance, MfaChallenge},
    EnrollmentResult, LoginChallenge, LoginChallengeIdentity, Principal, SessionGrant,
    SessionIdentity, SessionPolicy, SessionResult, SessionState, SessionStatus, UserRecord,
};
use crate::ApplicationError;

/// Inbound boundary consumed by delivery adapters.
pub trait IdentityWorkflow: Send + Sync {
    fn complete_totp_observed(&self, challenge_token: &str, code: &str) -> super::MfaAttempt {
        super::MfaAttempt::unspecified(self.complete_totp(challenge_token, code))
    }
    fn complete_recovery_observed(&self, challenge_token: &str, code: &str) -> super::MfaAttempt {
        super::MfaAttempt::unspecified(self.complete_recovery(challenge_token, code))
    }
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
    /// Reads current deadlines without treating a request as human activity.
    fn session_status(&self, access_token: &str) -> Result<SessionStatus, ApplicationError>;
    /// Records explicit activity only while the same session remains valid.
    fn record_activity(&self, access_token: &str) -> Result<SessionStatus, ApplicationError>;
    fn authorize(
        &self,
        access_token: &str,
        permission: Permission,
    ) -> Result<Principal, ApplicationError>;
    fn logout(&self, access_token: &str) -> Result<(), ApplicationError>;
}

/// Durable user persistence. Each mutation commits its audit event atomically.
pub trait UserRepository: Send + Sync {
    fn has_users(&self) -> Result<bool, ApplicationError>;
    /// Atomically inserts the first owner, or returns false if any user exists.
    fn insert_initial_owner(
        &self,
        user: UserRecord,
        at: OffsetDateTime,
    ) -> Result<bool, ApplicationError>;
    /// Rechecks the active owner under the same transaction as user creation.
    fn insert(
        &self,
        user: UserRecord,
        actor: UserId,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError>;
    fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, ApplicationError>;
    fn find_by_id(&self, id: UserId) -> Result<Option<UserRecord>, ApplicationError>;
    fn replace_recovery_codes(
        &self,
        id: UserId,
        expected_revision: u64,
        codes: RecoveryCodeSet,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError>;
}

/// Ephemeral challenges, sessions, throttling counters, and TOTP replay keys.
pub trait SessionStore: Send + Sync {
    fn create_challenge(
        &self,
        identity: &LoginChallengeIdentity,
        ttl: u64,
    ) -> Result<String, ApplicationError>;
    /// Atomically removes and returns a live challenge before any MFA attempt.
    /// Missing, expired, and already claimed challenges all return None.
    fn take_challenge(
        &self,
        token: &str,
    ) -> Result<Option<LoginChallengeIdentity>, ApplicationError>;
    /// Stores a strict certificate-origin MFA record with its absolute deadline.
    fn create_certificate_challenge(
        &self,
        value: &CertificateMfaChallenge,
    ) -> Result<String, ApplicationError>;
    /// Atomically claims either exact password JSON or a strict certificate record.
    /// Unknown or malformed origin must never be interpreted as Password.
    fn take_mfa_challenge(&self, token: &str) -> Result<Option<MfaChallenge>, ApplicationError>;
    fn create_session(
        &self,
        identity: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<SessionGrant, ApplicationError>;
    /// Atomically creates a certificate session capped by the supplied immutable ceiling.
    fn create_certificate_session(
        &self,
        identity: &SessionIdentity,
        provenance: &CertificateSessionProvenance,
        policy: SessionPolicy,
        ceiling_unix_ms: i64,
    ) -> Result<SessionGrant, ApplicationError>;
    /// Atomically rejects expired, malformed, or differently configured sessions.
    /// Reading never extends deadlines or repairs a missing expiration.
    fn find_session(
        &self,
        token: &str,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError>;
    /// Compares full identity and provenance before updating only idle time.
    fn record_certificate_activity(
        &self,
        token: &str,
        expected: &SessionIdentity,
        provenance: &CertificateSessionProvenance,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError>;
    /// Atomically checks identity and liveness before extending only idle time.
    /// Missing or revoked sessions remain absent and absolute time never grows.
    fn record_activity(
        &self,
        token: &str,
        expected: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError>;
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
