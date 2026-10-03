use domain::{
    clock::OffsetDateTime,
    crypto::{CredentialCheck, CredentialFailure},
    identity::UserId,
    owner_certificates::BindingRecord,
};

use crate::{credential_trust::CredentialTrustSnapshot, identity::Principal};

/// Neutral categories do not include submitted material or parser diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OwnerCertificateError {
    #[error("owner certificate input is invalid")]
    InvalidInput,
    #[error("owner certificate evidence is inconsistent")]
    Inconsistent,
    #[error("owner certificate account capture changed")]
    AccountChanged,
    #[error("published credential trust is unavailable")]
    TrustUnavailable,
    #[error("published credential trust changed")]
    TrustChanged,
    #[error("owner certificate registration identity conflicts")]
    BindingConflict,
    #[error("certificate fingerprint belongs to another account")]
    FingerprintConflict,
    #[error("owner already has a live certificate binding")]
    ActiveBinding,
    #[error("owner certificate binding was not found")]
    NotFound,
    #[error("owner certificate revision conflicts")]
    RevisionConflict,
    #[error("owner certificate material does not match")]
    MaterialMismatch,
    #[error("owner certificate verification failed: {0}")]
    Credential(#[from] CredentialFailure),
}

/// Current account facts contain no password, MFA secret or bearer token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerBindingAccount {
    pub principal: Principal,
    pub active: bool,
    pub revision: u64,
    pub auth_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerRegistrationContext {
    pub account: OwnerBindingAccount,
    pub trust: Option<CredentialTrustSnapshot>,
}

/// Immutable registration evidence and its optional terminal withdrawal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerBindingReceipt {
    pub owner: UserId,
    pub record: BindingRecord,
    pub check: CredentialCheck,
    pub trust: CredentialTrustSnapshot,
    pub registered_at: OffsetDateTime,
    pub withdrawn_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerWithdrawalContext {
    pub account: OwnerBindingAccount,
    pub receipt: OwnerBindingReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerBindingCommit {
    Applied(OwnerBindingReceipt),
    Existing(OwnerBindingReceipt),
}
