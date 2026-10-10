use domain::{
    clock::OffsetDateTime,
    crypto::{CredentialCheck, CredentialFailure},
    document_signing::{SignerAccount, SigningBindingStatement},
};

use crate::{credential_trust::CredentialTrustSnapshot, identity::Principal};

/// Neutral outcomes never contain submitted certificate or signature bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SigningWorkflowError {
    #[error("individual signing evidence is inconsistent")]
    Inconsistent,
    #[error("individual signing input is invalid")]
    Input,
    #[error("signing account capture changed")]
    AccountChanged,
    #[error("published credential trust is unavailable")]
    TrustUnavailable,
    #[error("published credential trust changed")]
    TrustChanged,
    #[error("individual signing intention conflicts")]
    Conflict,
    #[error("individual signing binding was not found")]
    NotFound,
    #[error("individual signing binding is inactive")]
    InactiveBinding,
    #[error("individual signing credential verification failed: {0}")]
    Credential(#[from] CredentialFailure),
}

/// Current account facts and current published trust, supplied by an authorized store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningAccountContext {
    pub principal: Principal,
    pub account: SignerAccount,
    pub trust: Option<CredentialTrustSnapshot>,
}

/// Immutable registration evidence with an optional terminal withdrawal time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningBindingReceipt {
    pub statement: SigningBindingStatement,
    pub check: CredentialCheck,
    pub trust: CredentialTrustSnapshot,
    pub registered_at: OffsetDateTime,
    pub withdrawn_at: Option<OffsetDateTime>,
}
