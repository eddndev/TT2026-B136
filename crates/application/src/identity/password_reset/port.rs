use super::{
    CompleteReset, IssueReset, ResetCandidate, ResetCompletion, ResetDeliveryOutcome,
    ResetEnvelope, ResetId, ResetIssueOutcome,
};
use crate::ApplicationError;
use domain::crypto::Sha256Digest;
use zeroize::Zeroizing;

/// Durable reset state and password changes share the existing audited transaction.
pub trait PasswordResetRepository: Send + Sync {
    /// Resolve an active account under the mutation lock and capture its generation.
    /// Enforce the live-capability quota without invalidating previous links.
    /// Determine issue time after locking and check expiry addition for overflow.
    /// Missing/inactive accounts and a full quota all return Ignored.
    fn issue(&self, command: IssueReset) -> Result<ResetIssueOutcome, ApplicationError>;

    /// Read a live capability's candidate without consuming it or renewing expiry.
    /// Expired, consumed, inactive and generation-invalid entries return None.
    fn inspect(&self, digest: Sha256Digest) -> Result<Option<ResetCandidate>, ApplicationError>;

    /// Match the observed issuance ID and digest, then recheck liveness, active
    /// identity, generation and counters after locking.
    /// Atomically replace only the password, increment revision and generation,
    /// consume the capability and append audit. Preserve current TOTP, remaining
    /// recovery codes, email, role and active state. Audit failure rolls back all.
    /// Rejections never mutate; uncertain commit failures remain errors.
    fn complete(&self, command: CompleteReset) -> Result<ResetCompletion, ApplicationError>;

    /// Cancel only this issuance and digest after a definite delivery rejection.
    /// Idempotent cancellation never affects account access or another capability.
    fn cancel_undelivered(&self, id: ResetId, digest: Sha256Digest)
        -> Result<(), ApplicationError>;
}

/// Delivery remains an internal dependency until a transport is configured.
pub trait PasswordResetDelivery: Send + Sync {
    /// A failure or uncertain response does not prove that the secret was undelivered.
    fn deliver(&self, envelope: ResetEnvelope) -> Result<ResetDeliveryOutcome, ApplicationError>;
}

/// Independent reset budgets; never read or reset password-login failure counts.
pub trait PasswordResetLimiter: Send + Sync {
    /// Apply before account lookup, using the existing normalized email form.
    fn admit_request(&self, normalized_email: &str) -> Result<bool, ApplicationError>;
    /// Apply before candidate lookup or expensive password hashing.
    fn admit_completion(&self, digest: Sha256Digest) -> Result<bool, ApplicationError>;
}

/// Generate exactly 256 bits of fresh cryptographic entropy for each capability.
pub trait ResetTokenSource: Send + Sync {
    fn generate(&self) -> Result<Zeroizing<[u8; 32]>, ApplicationError>;
}
