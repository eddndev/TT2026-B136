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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test double: issues the digest bytes as the "token" and accepts
    /// exactly that shape back when the anchor is non-empty. It only
    /// exercises the port signatures; it is not a real protocol.
    struct EchoAuthority;

    impl TimestampService for EchoAuthority {
        fn request(&self, digest: &Sha256Digest) -> Result<Vec<u8>, DomainError> {
            Ok(digest.as_bytes().to_vec())
        }
    }

    impl TimestampVerifier for EchoAuthority {
        fn verify(
            &self,
            token: &[u8],
            expected: &Sha256Digest,
            trust_anchor_pem: &[u8],
        ) -> Result<TimestampVerification, DomainError> {
            if token.len() != 32 {
                return Ok(TimestampVerification::MalformedToken(
                    "token must be 32 bytes".to_string(),
                ));
            }
            if token != expected.as_bytes() {
                return Ok(TimestampVerification::ImprintMismatch);
            }
            if trust_anchor_pem.is_empty() {
                return Ok(TimestampVerification::UntrustedToken(
                    "empty anchor".to_string(),
                ));
            }
            Ok(TimestampVerification::Valid {
                generated_at: "2026-01-01T00:00:00Z".to_string(),
            })
        }
    }

    #[test]
    fn ports_are_object_safe_and_round_trip() {
        let service: &dyn TimestampService = &EchoAuthority;
        let verifier: &dyn TimestampVerifier = &EchoAuthority;
        let digest = Sha256Digest::from_array([7u8; 32]);

        let token = service.request(&digest).unwrap();
        assert_eq!(
            verifier.verify(&token, &digest, b"anchor").unwrap(),
            TimestampVerification::Valid {
                generated_at: "2026-01-01T00:00:00Z".to_string(),
            }
        );
    }

    #[test]
    fn each_rejection_cause_is_reported_explicitly() {
        let verifier: &dyn TimestampVerifier = &EchoAuthority;
        let digest = Sha256Digest::from_array([7u8; 32]);
        let other = Sha256Digest::from_array([8u8; 32]);
        let token = digest.as_bytes().to_vec();

        assert_eq!(
            verifier.verify(&token, &other, b"anchor").unwrap(),
            TimestampVerification::ImprintMismatch
        );
        assert_eq!(
            verifier.verify(b"junk", &digest, b"anchor").unwrap(),
            TimestampVerification::MalformedToken("token must be 32 bytes".to_string())
        );
        assert_eq!(
            verifier.verify(&token, &digest, b"").unwrap(),
            TimestampVerification::UntrustedToken("empty anchor".to_string())
        );
    }

    #[test]
    fn authority_failure_error_displays_its_message() {
        let err = DomainError::TimestampAuthorityFailure("provider offline".to_string());
        assert_eq!(
            err.to_string(),
            "timestamp authority failure: provider offline"
        );
    }
}
