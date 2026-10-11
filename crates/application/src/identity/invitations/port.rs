use std::sync::Arc;

use domain::{
    clock::Clock,
    crypto::{DocumentHasher, PasswordHasher, RecoveryCodeGenerator, Sha256Digest, TotpProvider},
};
use zeroize::Zeroizing;

use crate::{
    identity::{
        password_reset::ResetTokenSource, IdentityWorkflow, SecretProtector, UserRepository,
    },
    ApplicationError,
};

use super::*;

pub trait InvitationWorkflow: Send + Sync {
    fn issue(
        &self,
        token: &str,
        request: InvitationIssueRequest,
    ) -> Result<InvitationSummary, ApplicationError>;
    fn resend(
        &self,
        token: &str,
        id: InvitationId,
        request: InvitationResendRequest,
    ) -> Result<InvitationSummary, ApplicationError>;
    fn list(&self, token: &str, query: InvitationQuery)
        -> Result<InvitationPage, ApplicationError>;
    fn get(&self, token: &str, id: InvitationId) -> Result<InvitationSummary, ApplicationError>;
    fn prepare(
        &self,
        token: &[u8],
        operation_id: Uuid,
        password: &str,
    ) -> Result<InvitationEnrollment, ApplicationError>;
    fn finish(
        &self,
        token: &[u8],
        operation_id: Uuid,
        password: &str,
        code: &str,
    ) -> Result<InvitationReceipt, ApplicationError>;
    fn receipt(
        &self,
        token: &[u8],
        operation_id: Uuid,
        password: &str,
    ) -> Result<Option<InvitationReceipt>, ApplicationError>;
}

/// Every Owner operation rechecks the complete InvitationActor under account
/// locks. UUID replay requires the exact normalized request; mismatch conflicts.
/// Existing active or inactive accounts are always UserAlreadyExists.
pub trait InvitationStore: Send + Sync {
    /// Atomically create R1, its encrypted delivery row and audit; expiry is
    /// exactly 24 hours from the committed issuance time after lock acquisition.
    fn issue(
        &self,
        actor: InvitationActor,
        command: IssueInvitation,
    ) -> Result<InvitationSummary, ApplicationError>;
    /// Any current Owner may resend. Atomically advance the expected revision,
    /// invalidate the previous digest and pending MFA, and enqueue fresh delivery.
    /// Accepted invitations cannot be resent. Exact UUID replay never rotates again.
    fn resend(
        &self,
        actor: InvitationActor,
        id: InvitationId,
        command: ResendInvitation,
    ) -> Result<InvitationSummary, ApplicationError>;
    fn list(
        &self,
        actor: InvitationActor,
        query: InvitationQuery,
    ) -> Result<InvitationPage, ApplicationError>;
    fn get(
        &self,
        actor: InvitationActor,
        id: InvitationId,
    ) -> Result<InvitationSummary, ApplicationError>;
    /// Missing, expired, superseded and consumed capabilities return None.
    fn inspect(
        &self,
        digest: Sha256Digest,
    ) -> Result<Option<InvitationCandidate>, ApplicationError>;
    /// Lock and recheck live digest/revision/expiry and absence of any account.
    /// Keep the first encrypted pending material. A racing/repeated preparation
    /// returns that same material; the application must verify its password.
    fn save_pending(
        &self,
        command: PrepareInvitation,
    ) -> Result<PendingInvitation, ApplicationError>;
    /// Recheck live digest, exact pending operation, revision, role/email and
    /// absence of any account. Create the user, consume capability, erase pending
    /// secrets, stop delivery, append audit and persist receipt in one transaction.
    /// Audit failure rolls everything back; uncertain commit must remain an error.
    /// Same operation and request_digest return the receipt; a mismatch conflicts.
    fn finish(&self, command: FinishInvitation) -> Result<InvitationReceipt, ApplicationError>;
    /// Consumed capability lookup returns metadata only. The application checks
    /// the recipient password against the resulting user's current password hash.
    fn completed(
        &self,
        digest: Sha256Digest,
        operation_id: Uuid,
    ) -> Result<Option<InvitationCompletion>, ApplicationError>;
    /// Claims only a live current revision, fencing old workers by generation.
    /// Retries retain delivery_id and stop at expiry or the first provider deadline.
    fn claim_delivery(
        &self,
        claim: InvitationDeliveryClaim,
    ) -> Result<Option<InvitationDeliveryLease>, ApplicationError>;
    /// Compare revision, generation and lease before persisting delivery state.
    /// Stale finishes return false. Retry time must fit the original provider window.
    fn finish_delivery(&self, result: InvitationDeliveryFinish) -> Result<bool, ApplicationError>;
}

/// Invitation-purpose ciphertext is never interchangeable with user TOTP storage.
/// Non-nil ID, positive signed-64-bit revision and bounded plaintext are mandatory.
pub trait InvitationProtector: Send + Sync {
    fn protect(
        &self,
        context: InvitationProtectionContext,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, ApplicationError>;
    fn expose(
        &self,
        context: InvitationProtectionContext,
        protected: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, ApplicationError>;
}

/// Independent invitation attempt budget; never alters login/reset counters.
pub trait InvitationLimiter: Send + Sync {
    fn admit(&self, digest: Sha256Digest) -> Result<bool, ApplicationError>;
}

pub struct InvitationPorts {
    pub identity: Arc<dyn IdentityWorkflow>,
    pub users: Arc<dyn UserRepository>,
    pub store: Arc<dyn InvitationStore>,
    pub protector: Arc<dyn InvitationProtector>,
    pub secrets: Arc<dyn SecretProtector>,
    pub limiter: Arc<dyn InvitationLimiter>,
    pub tokens: Arc<dyn ResetTokenSource>,
    pub digests: Arc<dyn DocumentHasher + Send + Sync>,
    pub passwords: Arc<dyn PasswordHasher + Send + Sync>,
    pub totp: Arc<dyn TotpProvider + Send + Sync>,
    pub recovery: Arc<dyn RecoveryCodeGenerator + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
}
