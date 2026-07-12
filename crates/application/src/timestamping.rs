//! Use cases: obtain and verify trusted timestamps over documents.
//!
//! A document travels as a stream and is digested through the hashing
//! port; the digest then meets the timestamping ports. The token is
//! opaque DER bytes end to end.

use std::io::Read;

use domain::crypto::timestamp::{TimestampService, TimestampVerification, TimestampVerifier};
use domain::crypto::{DocumentHasher, Sha256Digest};

use crate::error::ApplicationError;

/// A digest together with the timestamp token issued over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampedDigest {
    /// SHA-256 digest of the timestamped content.
    pub digest: Sha256Digest,
    /// DER-encoded timestamp token binding the digest to a time.
    pub token: Vec<u8>,
}

/// Streams a document through the hashing port and requests a
/// timestamp token over its digest.
pub struct TimestampDocument<H, T> {
    hasher: H,
    service: T,
}

impl<H: DocumentHasher, T: TimestampService> TimestampDocument<H, T> {
    /// Builds the use case over a hashing port and a timestamping port.
    pub fn new(hasher: H, service: T) -> Self {
        Self { hasher, service }
    }

    /// Digests `reader` to the end and obtains a token over the digest.
    pub fn execute(&self, reader: &mut dyn Read) -> Result<TimestampedDigest, ApplicationError> {
        let digest = self.hasher.hash_stream(reader)?;
        let token = self.service.request(&digest)?;
        Ok(TimestampedDigest { digest, token })
    }
}

/// Recomputes a document digest from its stream and checks a timestamp
/// token against it under the presented trust anchor.
pub struct VerifyTimestamp<H, V> {
    hasher: H,
    verifier: V,
}

impl<H: DocumentHasher, V: TimestampVerifier> VerifyTimestamp<H, V> {
    /// Builds the use case over a hashing port and a verification port.
    pub fn new(hasher: H, verifier: V) -> Self {
        Self { hasher, verifier }
    }

    /// Digests `reader` to the end and checks `token` over the result.
    pub fn execute(
        &self,
        reader: &mut dyn Read,
        token: &[u8],
        trust_anchor_pem: &[u8],
    ) -> Result<TimestampVerification, ApplicationError> {
        let digest = self.hasher.hash_stream(reader)?;
        Ok(self.verifier.verify(token, &digest, trust_anchor_pem)?)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use domain::crypto::timestamp::{TimestampService, TimestampVerification, TimestampVerifier};
    use domain::crypto::{DocumentHasher, Sha256Digest};
    use domain::DomainError;
    use mockall::mock;
    use mockall::predicate::eq;

    use super::{TimestampDocument, VerifyTimestamp};
    use crate::error::ApplicationError;

    mock! {
        Hasher {}

        impl DocumentHasher for Hasher {
            fn hash_bytes(&self, data: &[u8]) -> Sha256Digest;
            fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError>;
        }
    }

    mock! {
        Service {}

        impl TimestampService for Service {
            fn request(&self, digest: &Sha256Digest) -> Result<Vec<u8>, DomainError>;
        }
    }

    mock! {
        Verifier {}

        impl TimestampVerifier for Verifier {
            fn verify(
                &self,
                token: &[u8],
                expected: &Sha256Digest,
                trust_anchor_pem: &[u8],
            ) -> Result<TimestampVerification, DomainError>;
        }
    }

    fn digest() -> Sha256Digest {
        Sha256Digest::from_array([7u8; 32])
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
    fn timestamp_document_requests_a_token_over_the_streamed_digest() {
        let mut service = MockService::new();
        service
            .expect_request()
            .with(eq(digest()))
            .times(1)
            .returning(|_| Ok(vec![0x30, 0x82]));

        let use_case = TimestampDocument::new(stream_hasher(b"contract body"), service);
        let stamped = use_case.execute(&mut &b"contract body"[..]).unwrap();
        assert_eq!(stamped.digest, digest());
        assert_eq!(stamped.token, vec![0x30, 0x82]);
    }

    #[test]
    fn timestamp_document_surfaces_a_stream_read_failure() {
        let mut hasher = MockHasher::new();
        hasher.expect_hash_stream().times(1).returning(|_| {
            Err(DomainError::StreamRead {
                message: "broken pipe".to_string(),
            })
        });
        let mut service = MockService::new();
        service.expect_request().times(0);

        let use_case = TimestampDocument::new(hasher, service);
        let err = use_case.execute(&mut std::io::empty()).unwrap_err();
        assert!(matches!(
            err,
            ApplicationError::Domain(DomainError::StreamRead { .. })
        ));
    }

    #[test]
    fn timestamp_document_surfaces_an_authority_failure() {
        let mut service = MockService::new();
        service.expect_request().times(1).returning(|_| {
            Err(DomainError::TimestampAuthorityFailure(
                "provider offline".to_string(),
            ))
        });

        let use_case = TimestampDocument::new(stream_hasher(b""), service);
        let err = use_case.execute(&mut std::io::empty()).unwrap_err();
        assert!(matches!(
            err,
            ApplicationError::Domain(DomainError::TimestampAuthorityFailure(_))
        ));
    }

    #[test]
    fn verify_timestamp_checks_the_token_over_the_recomputed_digest() {
        let mut verifier = MockVerifier::new();
        verifier
            .expect_verify()
            .withf(|token, expected, anchor| {
                token == [0x30, 0x82] && *expected == digest() && anchor == b"root pem"
            })
            .times(1)
            .returning(|_, _, _| {
                Ok(TimestampVerification::Valid {
                    generated_at: "2026-07-12T00:00:00Z".to_string(),
                })
            });

        let use_case = VerifyTimestamp::new(stream_hasher(b"contract body"), verifier);
        let outcome = use_case
            .execute(&mut &b"contract body"[..], &[0x30, 0x82], b"root pem")
            .unwrap();
        assert_eq!(
            outcome,
            TimestampVerification::Valid {
                generated_at: "2026-07-12T00:00:00Z".to_string(),
            }
        );
    }

    #[test]
    fn verify_timestamp_passes_a_rejection_through_unchanged() {
        let mut verifier = MockVerifier::new();
        verifier
            .expect_verify()
            .times(1)
            .returning(|_, _, _| Ok(TimestampVerification::ImprintMismatch));

        let use_case = VerifyTimestamp::new(stream_hasher(b"altered"), verifier);
        let outcome = use_case
            .execute(&mut &b"altered"[..], &[0x01], b"root pem")
            .unwrap();
        assert_eq!(outcome, TimestampVerification::ImprintMismatch);
    }
}
