//! Shared mocks and fixtures for the document verification tests.
//!
//! Each test file compiles this module into its own crate, so not
//! every helper is used by every file; the allowance below silences
//! the resulting per-crate dead-code lint.
#![allow(dead_code)]

use std::io::Read;

use application::verification::{TimestampEvidence, VerifyDocumentRequest};
use domain::crypto::certificate::{
    CertificateSummary, CertificateValidation, CertificateValidator,
};
use domain::crypto::timestamp::{TimestampVerification, TimestampVerifier};
use domain::crypto::{
    DocumentHasher, Sha256Digest, Signature, SignatureVerification, SignatureVerifier,
};
use domain::DomainError;
use mockall::mock;

mock! {
    pub Hasher {}

    impl DocumentHasher for Hasher {
        fn hash_bytes(&self, data: &[u8]) -> Sha256Digest;
        fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError>;
    }
}

mock! {
    pub SigVerifier {}

    impl SignatureVerifier for SigVerifier {
        fn verify(
            &self,
            digest: &Sha256Digest,
            signature: &Signature,
            key_material: &[u8],
        ) -> Result<SignatureVerification, DomainError>;
    }
}

mock! {
    pub CertValidator {}

    impl CertificateValidator for CertValidator {
        fn validate<'a>(
            &self,
            certificate: &[u8],
            issuer: &[u8],
            crl: Option<&'a [u8]>,
            unix_seconds: i64,
        ) -> Result<CertificateValidation, DomainError>;
        fn inspect(&self, certificate: &[u8]) -> Result<CertificateSummary, DomainError>;
    }
}

mock! {
    pub TsVerifier {}

    impl TimestampVerifier for TsVerifier {
        fn verify(
            &self,
            token: &[u8],
            expected: &Sha256Digest,
            trust_anchor_pem: &[u8],
        ) -> Result<TimestampVerification, DomainError>;
    }
}

pub const EVALUATION_UNIX: i64 = 1_780_000_000;

pub fn digest() -> Sha256Digest {
    Sha256Digest::from_array([7u8; 32])
}

pub fn signature() -> Signature {
    Signature::from_bytes(vec![9u8; 4]).unwrap()
}

pub fn hasher() -> MockHasher {
    let mut hasher = MockHasher::new();
    hasher
        .expect_hash_stream()
        .times(1)
        .returning(|_| Ok(digest()));
    hasher
}

pub fn accepting_signature_verifier() -> MockSigVerifier {
    let mut verifier = MockSigVerifier::new();
    verifier
        .expect_verify()
        .times(1)
        .returning(|_, _, _| Ok(SignatureVerification::Valid));
    verifier
}

pub fn validator_reporting(outcome: CertificateValidation) -> MockCertValidator {
    let mut validator = MockCertValidator::new();
    validator
        .expect_validate()
        .times(1)
        .returning(move |_, _, _, _| Ok(outcome.clone()));
    validator
}

pub fn token_verifier_reporting(outcome: TimestampVerification) -> MockTsVerifier {
    let mut verifier = MockTsVerifier::new();
    verifier
        .expect_verify()
        .times(1)
        .returning(move |_, _, _| Ok(outcome.clone()));
    verifier
}

pub fn unused_token_verifier() -> MockTsVerifier {
    let mut verifier = MockTsVerifier::new();
    verifier.expect_verify().times(0);
    verifier
}

pub fn valid_token() -> TimestampVerification {
    TimestampVerification::Valid {
        generated_at: "2026-07-12T00:00:00Z".to_string(),
    }
}

pub fn full_request(sig: &Signature) -> VerifyDocumentRequest<'_> {
    VerifyDocumentRequest {
        signature: sig,
        signer_certificate_pem: b"signer pem",
        issuer_certificate_pem: b"issuer pem",
        crl_pem: Some(b"crl pem"),
        timestamp: Some(TimestampEvidence {
            token: b"token der",
            trust_anchor_pem: b"anchor pem",
        }),
        evaluation_unix: EVALUATION_UNIX,
    }
}
