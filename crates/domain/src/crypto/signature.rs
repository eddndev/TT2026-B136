//! Outbound ports for digital signatures over content digests.
//!
//! A document is never signed directly: the signer receives the SHA-256
//! digest of its content and produces a signature over that digest. The
//! verifier receives the digest, the signature bytes, and the signer's key
//! material (a certificate or a bare public key, as PEM bytes) and reports
//! an explicit outcome. The domain sees only bytes and outcomes; algorithm
//! and encoding choices live in the adapters.

use std::fmt;

use crate::crypto::digest::Sha256Digest;
use crate::error::DomainError;

/// A digital signature over a content digest.
///
/// The bytes are the raw signature value as produced by the signing
/// adapter, with no envelope or encoding around them.
#[derive(Clone, PartialEq, Eq)]
pub struct Signature(Vec<u8>);

impl Signature {
    /// Wraps raw signature bytes, rejecting an empty sequence.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, DomainError> {
        if bytes.is_empty() {
            return Err(DomainError::EmptySignature);
        }
        Ok(Self(bytes))
    }

    /// Borrows the raw signature bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consumes the signature, returning its bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

/// Signatures can be large; Debug prints the length instead of the bytes.
impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({} bytes)", self.0.len())
    }
}

/// Result of checking a signature against a digest and key material.
///
/// A rejected signature is a normal, expected outcome and therefore not an
/// error; the error path is reserved for backend failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureVerification {
    /// The signature matches the digest under the presented key material.
    Valid,
    /// The signature was rejected for the contained reason.
    Invalid(SignatureRejection),
}

/// Why a signature was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureRejection {
    /// The cryptographic check failed. A failed check cannot distinguish a
    /// changed document from a signature made with a different key, so both
    /// collapse into this single cause.
    MismatchedDocumentOrKey,
    /// The verifier key material could not be parsed or holds no usable
    /// public key.
    MalformedKeyMaterial(String),
    /// The signature bytes cannot belong to the presented key.
    MalformedSignature(String),
}

impl fmt::Display for SignatureRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedDocumentOrKey => {
                f.write_str("signature does not match document and key")
            }
            Self::MalformedKeyMaterial(message) => {
                write!(f, "malformed verifier key material: {message}")
            }
            Self::MalformedSignature(message) => write!(f, "malformed signature: {message}"),
        }
    }
}

/// Outbound port: produces signatures over content digests.
///
/// The adapter holds the private key material; callers never see it.
pub trait DocumentSigner {
    /// Signs `digest`, returning the raw signature bytes.
    fn sign(&self, digest: &Sha256Digest) -> Result<Signature, DomainError>;
}

/// Outbound port: checks signatures against digests and key material.
///
/// `key_material` carries the signer's certificate or bare public key as
/// PEM bytes; the adapter decides which forms it accepts.
pub trait SignatureVerifier {
    /// Checks `signature` over `digest` under `key_material`.
    ///
    /// Rejections are reported through [`SignatureVerification::Invalid`];
    /// the error path is reserved for backend failures.
    fn verify(
        &self,
        digest: &Sha256Digest,
        signature: &Signature,
        key_material: &[u8],
    ) -> Result<SignatureVerification, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_rejects_empty_bytes() {
        assert_eq!(
            Signature::from_bytes(Vec::new()).unwrap_err(),
            DomainError::EmptySignature
        );
    }

    #[test]
    fn signature_round_trips_its_bytes() {
        let bytes = vec![1u8, 2, 3];
        let signature = Signature::from_bytes(bytes.clone()).unwrap();
        assert_eq!(signature.as_bytes(), bytes.as_slice());
        assert_eq!(signature.into_bytes(), bytes);
    }

    #[test]
    fn signature_debug_hides_the_bytes() {
        let signature = Signature::from_bytes(vec![0xff; 4]).unwrap();
        assert_eq!(format!("{signature:?}"), "Signature(4 bytes)");
    }

    #[test]
    fn empty_signature_error_displays_its_message() {
        assert_eq!(
            DomainError::EmptySignature.to_string(),
            "signature must not be empty"
        );
    }

    #[test]
    fn rejection_causes_display_their_messages() {
        assert_eq!(
            SignatureRejection::MismatchedDocumentOrKey.to_string(),
            "signature does not match document and key"
        );
        assert_eq!(
            SignatureRejection::MalformedKeyMaterial("no pem".to_string()).to_string(),
            "malformed verifier key material: no pem"
        );
        assert_eq!(
            SignatureRejection::MalformedSignature("too short".to_string()).to_string(),
            "malformed signature: too short"
        );
    }

    /// Test double: signs by echoing the digest bytes as the signature and
    /// accepts exactly that shape back. It only exercises the port
    /// signatures; it is not a real scheme.
    struct EchoScheme;

    impl DocumentSigner for EchoScheme {
        fn sign(&self, digest: &Sha256Digest) -> Result<Signature, DomainError> {
            Signature::from_bytes(digest.as_bytes().to_vec())
        }
    }

    impl SignatureVerifier for EchoScheme {
        fn verify(
            &self,
            digest: &Sha256Digest,
            signature: &Signature,
            key_material: &[u8],
        ) -> Result<SignatureVerification, DomainError> {
            if key_material.is_empty() {
                return Ok(SignatureVerification::Invalid(
                    SignatureRejection::MalformedKeyMaterial("empty".to_string()),
                ));
            }
            if signature.as_bytes() == digest.as_bytes() {
                Ok(SignatureVerification::Valid)
            } else {
                Ok(SignatureVerification::Invalid(
                    SignatureRejection::MismatchedDocumentOrKey,
                ))
            }
        }
    }

    #[test]
    fn ports_are_object_safe_and_round_trip() {
        let signer: &dyn DocumentSigner = &EchoScheme;
        let verifier: &dyn SignatureVerifier = &EchoScheme;
        let digest = Sha256Digest::from_array([7u8; 32]);

        let signature = signer.sign(&digest).unwrap();
        assert_eq!(
            verifier.verify(&digest, &signature, b"key").unwrap(),
            SignatureVerification::Valid
        );

        let other = Sha256Digest::from_array([8u8; 32]);
        assert_eq!(
            verifier.verify(&other, &signature, b"key").unwrap(),
            SignatureVerification::Invalid(SignatureRejection::MismatchedDocumentOrKey)
        );
        assert_eq!(
            verifier.verify(&digest, &signature, b"").unwrap(),
            SignatureVerification::Invalid(SignatureRejection::MalformedKeyMaterial(
                "empty".to_string()
            ))
        );
    }
}
