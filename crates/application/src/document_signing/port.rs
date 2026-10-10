use std::sync::Arc;

use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::{
        CredentialCertificate, CredentialCheck, CredentialFailure, DocumentHasher, Signature,
    },
    document_signing::{DocumentConsent, SignerAccount, SigningBindingStatement, Uuid},
    identity::UserId,
};

use super::{
    SigningAccountContext, SigningBindingReceipt, SigningWorkflowError, VerifiedSigningBinding,
};
use crate::{
    credential_trust::CredentialTrustSnapshot,
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};

pub trait DocumentSigningVerifier: Send + Sync {
    fn inspect_certificate(
        &self,
        certificate: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure>;
    fn verify_binding(
        &self,
        statement: &SigningBindingStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, SigningWorkflowError>;
    fn verify_consent(
        &self,
        consent: &DocumentConsent,
        certificate: &[u8],
        pdf_signature: &Signature,
        consent_signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, SigningWorkflowError>;
}

/// All operations require the current active Litigator and expose only their binding.
///
/// Commits reauthorize the complete principal and counters under audit/account locks,
/// compare the complete current trust, then read time after every lock wait. One live
/// binding per account and permanent certificate fingerprint ownership are enforced.
/// Exact replay precedes current trust/time comparisons and returns its original receipt.
/// Different intentions conflict. Registration/withdrawal and their audits commit once
/// atomically. Withdrawal preserves evidence and requires no current certificate trust.
/// Port failures must never cause an implicit application retry.
pub trait SigningBindingStore: Send + Sync {
    fn load_context(&self, actor: UserId) -> Result<SigningAccountContext, ApplicationError>;
    fn find(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError>;
    fn find_current(
        &self,
        actor: UserId,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError>;
    fn commit_binding(
        &self,
        verified: VerifiedSigningBinding,
    ) -> Result<SigningBindingReceipt, ApplicationError>;
    fn withdraw(
        &self,
        principal: &Principal,
        binding: Uuid,
        expected: SignerAccount,
        at: OffsetDateTime,
    ) -> Result<SigningBindingReceipt, ApplicationError>;
}

pub struct SigningBindingPorts {
    pub identity: Arc<dyn IdentityWorkflow>,
    pub store: Arc<dyn SigningBindingStore>,
    pub verifier: Arc<dyn DocumentSigningVerifier>,
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
}
