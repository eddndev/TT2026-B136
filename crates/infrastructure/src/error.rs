//! Backend errors raised by adapters.

use application::ApplicationError;
use thiserror::Error;

/// Failure of a cryptographic adapter.
///
/// Covers the primitive operations offered by the hashing, encryption, and
/// signing adapters. Messages avoid echoing secret material.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// An authenticated-decryption check failed (wrong key, tag, or altered
    /// input).
    #[error("authenticated decryption failed")]
    DecryptionFailed,

    /// A key or nonce had an unexpected length.
    #[error("invalid key material: {0}")]
    InvalidKeyMaterial(String),

    /// The underlying cryptographic library reported a failure.
    #[error("cryptographic backend failure: {0}")]
    Backend(String),
}

/// Failure of the timestamp-authority adapter.
#[derive(Debug, Error)]
pub enum TsaError {
    /// The timestamp authority could not be reached.
    #[error("timestamp authority unreachable: {0}")]
    Unreachable(String),

    /// The timestamp authority rejected the request.
    #[error("timestamp authority rejected the request: {0}")]
    Rejected(String),

    /// The returned token could not be parsed or did not match the request.
    #[error("invalid timestamp token: {0}")]
    InvalidToken(String),
}

impl From<CryptoError> for ApplicationError {
    fn from(err: CryptoError) -> Self {
        ApplicationError::Port(err.to_string())
    }
}

impl From<TsaError> for ApplicationError {
    fn from(err: TsaError) -> Self {
        ApplicationError::Port(err.to_string())
    }
}
