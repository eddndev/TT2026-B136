//! Backend errors raised by adapters.

use application::ApplicationError;
use domain::DomainError;
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

/// Adapters implement domain ports, so their backend failures must surface
/// as domain errors. The decryption case maps onto the deliberately opaque
/// [`DomainError::AuthenticationFailed`].
impl From<CryptoError> for DomainError {
    fn from(err: CryptoError) -> Self {
        match err {
            CryptoError::DecryptionFailed => DomainError::AuthenticationFailed,
            CryptoError::InvalidKeyMaterial(msg) => DomainError::InvalidKeyMaterial(msg),
            CryptoError::Backend(msg) => DomainError::CryptoBackendFailure(msg),
        }
    }
}

impl From<TsaError> for ApplicationError {
    fn from(err: TsaError) -> Self {
        ApplicationError::Port(err.to_string())
    }
}

/// Timestamp adapters implement the domain timestamping port, so their
/// failures must surface as domain errors; the adapter's diagnosis is
/// carried in the message.
impl From<TsaError> for DomainError {
    fn from(err: TsaError) -> Self {
        DomainError::TimestampAuthorityFailure(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crypto_error_becomes_an_application_port_error() {
        let err: ApplicationError = CryptoError::Backend("openssl absent".to_string()).into();
        assert!(
            matches!(err, ApplicationError::Port(ref message) if message.contains("openssl absent")),
            "a crypto error must map to an application port error"
        );
    }

    #[test]
    fn a_crypto_backend_error_becomes_a_domain_backend_failure() {
        let err: DomainError = CryptoError::Backend("boom".to_string()).into();
        assert_eq!(err, DomainError::CryptoBackendFailure("boom".to_string()));
    }

    #[test]
    fn a_tsa_error_becomes_an_application_port_error() {
        let err: ApplicationError = TsaError::Unreachable("host down".to_string()).into();
        assert!(
            matches!(err, ApplicationError::Port(ref message) if message.contains("host down")),
            "a tsa error must map to an application port error"
        );
    }
}
