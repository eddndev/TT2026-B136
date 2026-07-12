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
}
