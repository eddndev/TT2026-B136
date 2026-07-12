//! Use cases: sign and verify documents and login challenges.
//!
//! A document travels as a stream and is digested through the hashing port;
//! a challenge is a short in-memory byte string and is digested directly.
//! Both paths then meet the same signing and verification ports, so a
//! challenge signature is checked exactly like a document signature.

use std::io::Read;

use domain::crypto::{
    DocumentHasher, DocumentSigner, Sha256Digest, Signature, SignatureVerification,
    SignatureVerifier,
};

use crate::error::ApplicationError;

/// A digest together with the signature produced over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedDigest {
    /// SHA-256 digest of the signed content.
    pub digest: Sha256Digest,
    /// Signature over that digest.
    pub signature: Signature,
}

/// Streams a document through the hashing port and signs its digest.
pub struct SignDocument<H, S> {
    hasher: H,
    signer: S,
}

impl<H: DocumentHasher, S: DocumentSigner> SignDocument<H, S> {
    /// Builds the use case over a hashing port and a signing port.
    pub fn new(hasher: H, signer: S) -> Self {
        Self { hasher, signer }
    }

    /// Digests `reader` to the end and signs the digest.
    pub fn execute(&self, reader: &mut dyn Read) -> Result<SignedDigest, ApplicationError> {
        let digest = self.hasher.hash_stream(reader)?;
        let signature = self.signer.sign(&digest)?;
        Ok(SignedDigest { digest, signature })
    }
}

/// Recomputes a document digest from its stream and checks a signature
/// against it under the presented key material.
pub struct VerifySignature<H, V> {
    hasher: H,
    verifier: V,
}

impl<H: DocumentHasher, V: SignatureVerifier> VerifySignature<H, V> {
    /// Builds the use case over a hashing port and a verification port.
    pub fn new(hasher: H, verifier: V) -> Self {
        Self { hasher, verifier }
    }

    /// Digests `reader` to the end and checks `signature` over the result.
    pub fn execute(
        &self,
        reader: &mut dyn Read,
        signature: &Signature,
        key_material: &[u8],
    ) -> Result<SignatureVerification, ApplicationError> {
        let digest = self.hasher.hash_stream(reader)?;
        Ok(self.verifier.verify(&digest, signature, key_material)?)
    }
}

/// Signs a short challenge byte string, the groundwork for a login that
/// proves possession of a certificate's private key.
pub struct SignChallenge<H, S> {
    hasher: H,
    signer: S,
}

impl<H: DocumentHasher, S: DocumentSigner> SignChallenge<H, S> {
    /// Builds the use case over a hashing port and a signing port.
    pub fn new(hasher: H, signer: S) -> Self {
        Self { hasher, signer }
    }

    /// Digests `challenge` and signs the digest.
    pub fn execute(&self, challenge: &[u8]) -> Result<SignedDigest, ApplicationError> {
        let digest = self.hasher.hash_bytes(challenge);
        let signature = self.signer.sign(&digest)?;
        Ok(SignedDigest { digest, signature })
    }
}

/// Checks a signature over a short challenge byte string.
pub struct VerifyChallenge<H, V> {
    hasher: H,
    verifier: V,
}

impl<H: DocumentHasher, V: SignatureVerifier> VerifyChallenge<H, V> {
    /// Builds the use case over a hashing port and a verification port.
    pub fn new(hasher: H, verifier: V) -> Self {
        Self { hasher, verifier }
    }

    /// Digests `challenge` and checks `signature` over the result.
    pub fn execute(
        &self,
        challenge: &[u8],
        signature: &Signature,
        key_material: &[u8],
    ) -> Result<SignatureVerification, ApplicationError> {
        let digest = self.hasher.hash_bytes(challenge);
        Ok(self.verifier.verify(&digest, signature, key_material)?)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use domain::crypto::{
        DocumentHasher, DocumentSigner, Sha256Digest, Signature, SignatureRejection,
        SignatureVerification, SignatureVerifier,
    };
    use domain::DomainError;
    use mockall::mock;
    use mockall::predicate::eq;

    use super::{SignChallenge, SignDocument, VerifyChallenge, VerifySignature};
    use crate::error::ApplicationError;

    mock! {
        Hasher {}

        impl DocumentHasher for Hasher {
            fn hash_bytes(&self, data: &[u8]) -> Sha256Digest;
            fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError>;
        }
    }

    mock! {
        Signer {}

        impl DocumentSigner for Signer {
            fn sign(&self, digest: &Sha256Digest) -> Result<Signature, DomainError>;
        }
    }

    mock! {
        Verifier {}

        impl SignatureVerifier for Verifier {
            fn verify(
                &self,
                digest: &Sha256Digest,
                signature: &Signature,
                key_material: &[u8],
            ) -> Result<SignatureVerification, DomainError>;
        }
    }

    fn digest() -> Sha256Digest {
        Sha256Digest::from_array([7u8; 32])
    }

    fn signature() -> Signature {
        Signature::from_bytes(vec![9u8; 4]).unwrap()
    }

    fn stream_hasher(expected: &'static [u8]) -> MockHasher {
        let mut hasher = MockHasher::new();
        hasher
            .expect_hash_stream()
            .times(1)
            .returning(move |reader| {
                let mut consumed = Vec::new();
                reader.read_to_end(&mut consumed).unwrap();
                assert_eq!(consumed, expected);
                Ok(digest())
            });
        hasher
    }

    #[test]
    fn sign_document_signs_the_digest_of_the_streamed_content() {
        let mut signer = MockSigner::new();
        signer
            .expect_sign()
            .with(eq(digest()))
            .times(1)
            .returning(|_| Ok(signature()));

        let use_case = SignDocument::new(stream_hasher(b"contract body"), signer);
        let signed = use_case.execute(&mut &b"contract body"[..]).unwrap();
        assert_eq!(signed.digest, digest());
        assert_eq!(signed.signature, signature());
    }

    #[test]
    fn sign_document_surfaces_a_stream_read_failure() {
        let mut hasher = MockHasher::new();
        hasher.expect_hash_stream().times(1).returning(|_| {
            Err(DomainError::StreamRead {
                message: "broken pipe".to_string(),
            })
        });
        let mut signer = MockSigner::new();
        signer.expect_sign().times(0);

        let use_case = SignDocument::new(hasher, signer);
        let err = use_case.execute(&mut std::io::empty()).unwrap_err();
        assert!(matches!(
            err,
            ApplicationError::Domain(DomainError::StreamRead { .. })
        ));
    }

    #[test]
    fn sign_document_surfaces_a_signer_failure() {
        let mut signer = MockSigner::new();
        signer
            .expect_sign()
            .times(1)
            .returning(|_| Err(DomainError::CryptoBackendFailure("rsa broke".to_string())));

        let use_case = SignDocument::new(stream_hasher(b""), signer);
        let err = use_case.execute(&mut std::io::empty()).unwrap_err();
        assert!(matches!(
            err,
            ApplicationError::Domain(DomainError::CryptoBackendFailure(_))
        ));
    }

    #[test]
    fn verify_signature_checks_the_recomputed_digest_under_the_key_material() {
        let mut verifier = MockVerifier::new();
        verifier
            .expect_verify()
            .withf(|d, s, material| {
                *d == digest() && *s == signature() && material == b"certificate pem"
            })
            .times(1)
            .returning(|_, _, _| Ok(SignatureVerification::Valid));

        let use_case = VerifySignature::new(stream_hasher(b"contract body"), verifier);
        let outcome = use_case
            .execute(&mut &b"contract body"[..], &signature(), b"certificate pem")
            .unwrap();
        assert_eq!(outcome, SignatureVerification::Valid);
    }

    #[test]
    fn verify_signature_passes_a_rejection_through_unchanged() {
        let mut verifier = MockVerifier::new();
        verifier.expect_verify().times(1).returning(|_, _, _| {
            Ok(SignatureVerification::Invalid(
                SignatureRejection::MismatchedDocumentOrKey,
            ))
        });

        let use_case = VerifySignature::new(stream_hasher(b"altered"), verifier);
        let outcome = use_case
            .execute(&mut &b"altered"[..], &signature(), b"certificate pem")
            .unwrap();
        assert_eq!(
            outcome,
            SignatureVerification::Invalid(SignatureRejection::MismatchedDocumentOrKey)
        );
    }

    #[test]
    fn sign_challenge_signs_the_digest_of_the_challenge_bytes() {
        let mut hasher = MockHasher::new();
        hasher
            .expect_hash_bytes()
            .with(eq(b"login nonce".as_slice()))
            .times(1)
            .returning(|_| digest());
        let mut signer = MockSigner::new();
        signer
            .expect_sign()
            .with(eq(digest()))
            .times(1)
            .returning(|_| Ok(signature()));

        let use_case = SignChallenge::new(hasher, signer);
        let signed = use_case.execute(b"login nonce").unwrap();
        assert_eq!(signed.digest, digest());
        assert_eq!(signed.signature, signature());
    }

    #[test]
    fn verify_challenge_checks_the_challenge_digest_under_the_key_material() {
        let mut hasher = MockHasher::new();
        hasher
            .expect_hash_bytes()
            .with(eq(b"login nonce".as_slice()))
            .times(1)
            .returning(|_| digest());
        let mut verifier = MockVerifier::new();
        verifier
            .expect_verify()
            .withf(|d, s, material| *d == digest() && *s == signature() && material == b"spki pem")
            .times(1)
            .returning(|_, _, _| Ok(SignatureVerification::Valid));

        let use_case = VerifyChallenge::new(hasher, verifier);
        let outcome = use_case
            .execute(b"login nonce", &signature(), b"spki pem")
            .unwrap();
        assert_eq!(outcome, SignatureVerification::Valid);
    }
}
