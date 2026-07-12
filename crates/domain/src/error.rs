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
}
