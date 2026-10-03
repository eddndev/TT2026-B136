use std::sync::Arc;

use domain::{
    clock::Clock,
    crypto::{
        CredentialCertificate, CredentialCheck, CredentialFailure, DocumentHasher, Signature,
    },
    identity::UserId,
    owner_certificates::{BindingStatement, Uuid},
};

use super::{
    OwnerBindingCommit, OwnerBindingReceipt, OwnerCertificateError, OwnerRegistrationContext,
    OwnerWithdrawalContext, PreparedOwnerWithdrawal, VerifiedOwnerRegistration,
};
use crate::{
    credential_trust::CredentialTrustSnapshot, identity::IdentityWorkflow, ApplicationError,
};

pub trait OwnerBindingVerifier: Send + Sync {
    fn inspect_certificate(
        &self,
        certificate: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure>;
    fn verify_registration(
        &self,
        statement: &BindingStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, OwnerCertificateError>;
}

/// Every read requires the current active Owner before returning any history.
///
/// Commit implementations reauthorize under audit/account/binding locks. New
/// registrations recheck exact counters, full current trust and time after all
/// waits, enforce UUID intent equality, one live binding per Owner and permanent
/// fingerprint ownership, and atomically append the exact mutation audit event.
/// Withdrawals require current authority, counters, UUID, revision and time,
/// but never current trust or certificate validity. Neither operation changes
/// account generations or MFA. A commit error must not trigger an implicit retry.
pub trait OwnerCertificateStore: Send + Sync {
    fn load_registration(
        &self,
        actor: UserId,
    ) -> Result<OwnerRegistrationContext, ApplicationError>;
    fn find(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<Option<OwnerBindingReceipt>, ApplicationError>;
    fn commit_registration(
        &self,
        verified: VerifiedOwnerRegistration,
    ) -> Result<OwnerBindingCommit, ApplicationError>;
    fn load_withdrawal(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<OwnerWithdrawalContext, ApplicationError>;
    /// Existing must return the original terminal receipt for the same exact
    /// registration, retaining its withdrawal counters and acceptance time.
    fn commit_withdrawal(
        &self,
        prepared: PreparedOwnerWithdrawal,
    ) -> Result<OwnerBindingCommit, ApplicationError>;
}

pub struct OwnerCertificatePorts {
    pub identity: Arc<dyn IdentityWorkflow>,
    pub store: Arc<dyn OwnerCertificateStore>,
    pub verifier: Arc<dyn OwnerBindingVerifier>,
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
}
