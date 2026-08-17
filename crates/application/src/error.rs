//! Errors surfaced by use cases.

use domain::DomainError;
use thiserror::Error;

/// Failure of a use case.
///
/// A use case either violates a domain invariant (wrapped from
/// [`DomainError`]) or fails while talking to an outbound port. Port failures
/// are reported as a message so this crate stays free of adapter details.
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// A domain invariant was violated.
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// An outbound port reported a failure.
    #[error("port failure: {0}")]
    Port(String),

    /// Bytes presented as a vault file did not match the vault file format
    /// documented in the `vault` module.
    #[error("malformed vault file: {0}")]
    MalformedVaultFile(String),

    /// A freshly issued certificate failed the validation that runs
    /// before issuance is reported as successful; the message carries the
    /// validation outcome.
    #[error("issued certificate failed post-issuance validation: {0}")]
    IssuedCertificateInvalid(String),

    /// A requested document identity has no stored record.
    #[error("document not found: {0}")]
    DocumentNotFound(String),

    /// A repository already contains the identity being inserted.
    #[error("document already exists: {0}")]
    DocumentAlreadyExists(String),

    /// A document already has immutable signature and timestamp evidence.
    #[error("document is already sealed: {0}")]
    DocumentAlreadySealed(String),

    /// An operation requires evidence that has not been created yet.
    #[error("document is not sealed: {0}")]
    DocumentNotSealed(String),

    /// Stored metadata contradicts the encrypted document content.
    #[error("stored document is inconsistent: {0}")]
    StoredDocumentInconsistent(String),

    /// Runtime material cannot satisfy the workflow's fixed contracts.
    #[error("invalid application configuration: {0}")]
    InvalidConfiguration(String),

    /// Freshly produced evidence failed its immediate integrity check.
    #[error("timestamp evidence was rejected: {0}")]
    TimestampEvidenceRejected(String),

    /// A caller supplied an unusable application-level value.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
