//! Outbound ports for trusted timestamping (RFC 3161).
//!
//! A timestamp token is the DER-encoded artifact returned by a timestamp
//! authority; it binds a SHA-256 digest to a time assertion signed by that
//! authority. The domain handles the token as opaque bytes and reports
//! explicit verification outcomes; protocol and encoding details live in
//! the adapters.

use crate::crypto::digest::Sha256Digest;
use crate::error::DomainError;

/// Outcome of checking a timestamp token against an expected digest and a
/// trust anchor.
///
/// A rejected token is a normal, expected outcome and therefore not an
/// error; the error path is reserved for backend failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimestampVerification {
    /// The token is authentic under the trust anchor and covers the
    /// expected digest.
    Valid {
        /// Generation time asserted by the authority, as RFC 3339 text.
        generated_at: String,
    },
    /// The token covers a different digest or digest algorithm than the
    /// expected one.
    ImprintMismatch,
    /// The bytes could not be decoded as a timestamp token; the message
    /// carries the decoder's diagnosis.
    MalformedToken(String),
    /// The token decoded correctly but its signature or the authority's
    /// certificate chain failed against the presented trust anchor; the
    /// message carries the verifier's diagnosis.
    UntrustedToken(String),
}

/// Outbound port: obtains timestamp tokens from a timestamp authority.
///
/// The adapter decides how the authority is reached (a remote provider, a
/// local development authority); callers only see digest in, token out.
pub trait TimestampService {
    /// Requests a token binding `digest` to the authority's current time.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::TimestampAuthorityFailure`] when the
    /// authority cannot be reached, rejects the request, or returns an
    /// unusable response.
    fn request(&self, digest: &Sha256Digest) -> Result<Vec<u8>, DomainError>;
}

/// Outbound port: checks timestamp tokens against digests and anchors.
///
/// `trust_anchor_pem` carries the certificate (as PEM bytes) that the
/// authority's certificate chain must lead to.
pub trait TimestampVerifier {
    /// Checks `token` over the `expected` digest under `trust_anchor_pem`.
    ///
    /// Rejections are reported through the non-`Valid` variants of
    /// [`TimestampVerification`]; the error path is reserved for backend
    /// failures.
    fn verify(
        &self,
        token: &[u8],
        expected: &Sha256Digest,
        trust_anchor_pem: &[u8],
    ) -> Result<TimestampVerification, DomainError>;
}
