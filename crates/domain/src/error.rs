//! Errors raised when domain invariants are violated.

use thiserror::Error;

/// Failure to build or operate on a domain value.
///
/// Variants describe violations of value-object invariants. They carry enough
/// detail for a caller to report the cause without inspecting internals.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    /// A digest was built from a byte slice of the wrong length.
    #[error("digest must be {expected} bytes, got {actual}")]
    InvalidDigestLength { expected: usize, actual: usize },

    /// A hex string could not be decoded into the expected byte length.
    #[error("invalid hex encoding for a {expected}-byte value")]
    InvalidHexEncoding { expected: usize },

    /// A document version was outside the allowed range (versions start at 1).
    #[error("document version must be 1 or greater")]
    InvalidDocumentVersion,

    /// Reading from an input stream failed while digesting its content.
    #[error("failed to read input stream: {message}")]
    StreamRead { message: String },

    /// An authenticated decryption was rejected. Deliberately opaque: it
    /// does not reveal whether the key, the tag, or the data was wrong.
    #[error("authenticated decryption failed")]
    AuthenticationFailed,

    /// Key material was unusable: wrong length or rejected by the backend.
    #[error("invalid key material: {0}")]
    InvalidKeyMaterial(String),

    /// A sealed payload was too short to hold a nonce and a tag.
    #[error("sealed payload must be at least {min} bytes, got {actual}")]
    MalformedSealedPayload { min: usize, actual: usize },

    /// The cryptographic backend failed for a reason other than a rejected
    /// authentication check.
    #[error("cryptographic backend failure: {0}")]
    CryptoBackendFailure(String),
}
