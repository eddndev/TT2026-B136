//! Individual public certificate bindings and externally signed document intentions.

mod binding_service;
mod document_manifest;
mod document_model;
mod document_port;
mod document_service;
mod document_validation;
mod document_verification;
mod evidence;
mod manifest_port;
mod model;
mod port;
mod prepared;

pub use binding_service::SigningBindingService;
pub use document_model::{
    CurrentSigningStatus, PreparedDocumentSigning, SigningCommit, SigningReceipt,
    SigningVerification, SigningVerificationContext, VerifiedDocumentSigning,
};
pub use document_port::{DocumentSigningPorts, IndividualSigningStore};
pub use document_service::DocumentSigningService;
pub use document_validation::validate_signing_receipt;
pub use evidence::validate_binding_receipt;
pub use manifest_port::{ManifestTimestamping, TokenFacts, TsaAuthority};
pub use model::{SigningAccountContext, SigningBindingReceipt, SigningWorkflowError};
pub use port::{DocumentSigningVerifier, SigningBindingPorts, SigningBindingStore};
pub use prepared::{PreparedSigningBinding, VerifiedSigningBinding};

/// Checks shared with bilateral contract signing, which uses the same
/// bindings, trust snapshots, PEM encoding and time-stamping anchor.
pub(crate) use document_manifest::{check_anchor, check_before};
pub(crate) use document_validation::{pem, successor as check_successor};
pub(crate) use evidence::{check_time, trust as check_trust};
