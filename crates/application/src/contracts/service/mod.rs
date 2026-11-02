//! The contract use cases. Every operation authenticates the session,
//! reads the stored contract with the caller's identity, applies the access
//! and state rules, and authenticates again before returning. Rationale:
//! `docs/adr/0086-bilateral-contract-signing.md`.

mod close;
mod content;
mod export;
mod lifecycle;
mod reads;
mod request;
mod sign;
mod sign_firm;
mod sign_rules;

use std::sync::Arc;

use domain::document_signing::Uuid;
use domain::{
    clock::Clock,
    crypto::{ArchiveWriter, DocumentHasher},
};

use super::{
    error::ContractWorkflowError as Error,
    model::{ContractReceipt, ContractSnapshot},
    notices::NoticeSettings,
    port::{ContractSignatureVerifier, ContractStore},
};
use crate::{
    document_integrity::DocumentIntegrityStore,
    document_signing::{ManifestTimestamping, SigningBindingStore},
    documents::{DocumentFormatBatchValidator, DocumentProcessor},
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};

pub use close::CloseOutcome;
pub use content::ContractContent;
pub use export::{ContractPackage, ContractStage};
pub use reads::{CaseContracts, ContractView, RequestView};
pub use request::NewRequest;
pub use sign::{PreparedContractSigning, SubmittedSignature};

pub struct ContractPorts {
    pub identity: Arc<dyn IdentityWorkflow>,
    pub store: Arc<dyn ContractStore>,
    pub bindings: Arc<dyn SigningBindingStore>,
    pub verifier: Arc<dyn ContractSignatureVerifier>,
    /// Requests T1, Td and Tf once each, never retrying.
    pub timestamps: Arc<dyn ManifestTimestamping>,
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    /// Decrypts exact content and supplies the time-stamping anchor, which
    /// must be the credential trust root.
    pub processor: Arc<DocumentProcessor>,
    pub formats: Arc<dyn DocumentFormatBatchValidator>,
    pub incidents: Arc<dyn DocumentIntegrityStore>,
    pub archiver: Arc<dyn ArchiveWriter + Send + Sync>,
    pub notices: NoticeSettings,
}

pub struct ContractService {
    pub(super) ports: ContractPorts,
}

impl ContractService {
    pub fn new(ports: ContractPorts) -> Self {
        Self { ports }
    }

    pub(super) fn principal(&self, token: &str) -> Result<Principal, ApplicationError> {
        self.ports.identity.authenticate(token)
    }

    pub(super) fn reauthenticate(
        &self,
        token: &str,
        expected: &Principal,
    ) -> Result<(), ApplicationError> {
        if self.ports.identity.authenticate(token)? != *expected {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }

    pub(super) fn now(&self) -> domain::clock::OffsetDateTime {
        self.ports.clock.now()
    }

    /// The contract holding `request`, or `contract_not_found`.
    pub(super) fn by_request(
        &self,
        principal: &Principal,
        request: Uuid,
    ) -> Result<ContractSnapshot, ApplicationError> {
        if request.is_nil() {
            return Err(Error::NotFound.into());
        }
        let snapshot = self
            .ports
            .store
            .contract_by_request(principal, request)?
            .ok_or(Error::NotFound)?;
        if snapshot.request(request).is_none() {
            return Err(Error::Inconsistent.into());
        }
        Ok(snapshot)
    }

    /// The receipt of `operation`, or `contract_not_found`.
    pub(super) fn find_receipt(
        &self,
        principal: &Principal,
        operation: Uuid,
    ) -> Result<Option<ContractReceipt>, ApplicationError> {
        if operation.is_nil() {
            return Err(Error::Input.into());
        }
        self.ports.store.receipt(principal, operation)
    }
}

/// Rejects a nil identifier with `contract_input`.
pub(super) fn identity(value: Uuid) -> Result<Uuid, ApplicationError> {
    if value.is_nil() {
        return Err(Error::Input.into());
    }
    Ok(value)
}

/// Rejects `expected` when it differs from the stored revision.
pub(super) fn expect_revision(
    snapshot: &ContractSnapshot,
    expected: u32,
) -> Result<u32, ApplicationError> {
    if snapshot.revision() != expected {
        return Err(Error::RevisionConflict.into());
    }
    expected
        .checked_add(1)
        .ok_or_else(|| Error::Inconsistent.into())
}

/// Text of 1 to `max` characters with no control character.
pub(super) fn bounded_text(value: &str, max: usize) -> Result<String, ApplicationError> {
    let count = value.chars().count();
    if value.trim().is_empty() || count > max || value.chars().any(char::is_control) {
        return Err(Error::Input.into());
    }
    Ok(value.to_owned())
}
