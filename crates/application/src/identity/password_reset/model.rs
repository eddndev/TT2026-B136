use crate::ApplicationError;
use domain::clock::OffsetDateTime;
use domain::crypto::Sha256Digest;
use domain::identity::{ResetId, UserId};
use zeroize::{Zeroize, Zeroizing};

/// Explicit issuance policy. There is no operational default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetPolicy {
    ttl_seconds: u64,
    max_pending_per_account: u32,
}

impl ResetPolicy {
    pub fn new(ttl_seconds: u64, max_pending_per_account: u32) -> Result<Self, ApplicationError> {
        let invalid = || {
            ApplicationError::InvalidConfiguration(
                "password reset requires a representable positive lifetime and capacity".into(),
            )
        };
        let seconds = i64::try_from(ttl_seconds).map_err(|_| invalid())?;
        if seconds == 0
            || max_pending_per_account == 0
            || OffsetDateTime::UNIX_EPOCH
                .checked_add(time::Duration::seconds(seconds))
                .is_none()
        {
            return Err(invalid());
        }
        Ok(Self {
            ttl_seconds,
            max_pending_per_account,
        })
    }

    pub const fn ttl_seconds(self) -> u64 {
        self.ttl_seconds
    }

    pub const fn max_pending_per_account(self) -> u32 {
        self.max_pending_per_account
    }
}

/// The same response for issued, ignored and throttled requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetRequestAccepted;

/// Durable issuance input. Only the repository resolves account identity and time.
pub struct IssueReset {
    pub email: String,
    pub digest: Sha256Digest,
    pub policy: ResetPolicy,
}

/// Internal delivery coordinates returned after persistence is confirmed.
pub struct ResetIssue {
    pub id: ResetId,
    pub email: String,
    pub expires_at: OffsetDateTime,
}

pub enum ResetIssueOutcome {
    Issued(ResetIssue),
    Ignored,
}

/// A provisional observation that the consuming transaction must revalidate.
#[derive(Clone)]
pub struct ResetCandidate {
    pub id: ResetId,
    pub user_id: UserId,
    pub auth_generation: u64,
}

/// One atomic password mutation, including capability consumption and audit.
/// The hash is produced by PasswordHasher, never accepted from a public caller.
pub struct CompleteReset {
    pub digest: Sha256Digest,
    pub candidate: ResetCandidate,
    pub password_hash: String,
}

impl Drop for CompleteReset {
    fn drop(&mut self) {
        self.password_hash.zeroize();
    }
}

/// No session or MFA material is issued by password recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetCompletion {
    Changed,
    Rejected,
}

/// Sensitive delivery payload. Adapters must not log or persist the token in clear.
pub struct ResetEnvelope {
    pub email: String,
    pub token: Zeroizing<[u8; 32]>,
    pub expires_at: OffsetDateTime,
}

/// Only a definite rejection permits cancelling the exact newly issued capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetDeliveryOutcome {
    Accepted,
    DefinitelyRejected,
    Uncertain,
}
