use application::{
    documents::{
        DocumentFormatBatch, DocumentFormatBatchValidator, DocumentProcessor,
        DocumentProcessorPorts, EvidenceMaterial, StageDocumentFormat,
    },
    ApplicationError,
};
use domain::{
    crypto::{
        DocumentSigner, Sha256Digest, Signature, TimestampService, TimestampVerification,
        TimestampVerifier,
    },
    DomainError,
};
use infrastructure::{
    EnvelopeKeyManager, RingAesGcmCipher, RingSha256Hasher, RsaPkcs1Verifier, StoredZipWriter,
    X509ChainValidator,
};
use zeroize::Zeroizing;

struct ForbiddenSigner;
impl DocumentSigner for ForbiddenSigner {
    fn sign(&self, _: &Sha256Digest) -> Result<Signature, DomainError> {
        panic!("no global signing")
    }
}
struct Timestamp;
impl TimestampService for Timestamp {
    fn request(&self, digest: &Sha256Digest) -> Result<Vec<u8>, DomainError> {
        Ok(digest.as_bytes().to_vec())
    }
}
impl TimestampVerifier for Timestamp {
    fn verify(
        &self,
        token: &[u8],
        digest: &Sha256Digest,
        _: &[u8],
    ) -> Result<TimestampVerification, DomainError> {
        assert_eq!(token, digest.as_bytes());
        Ok(TimestampVerification::Valid {
            generated_at: "2026-10-10T20:00:00Z".into(),
        })
    }
}
pub struct Pdf;
impl DocumentFormatBatchValidator for Pdf {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        assert_eq!(batch.inputs().len(), 1);
        Ok(vec![StageDocumentFormat::Pdf])
    }
}
pub fn processor() -> DocumentProcessor {
    DocumentProcessor::new(
        DocumentProcessorPorts {
            hasher: Box::new(RingSha256Hasher),
            cipher: Box::new(RingAesGcmCipher::new()),
            keys: Box::new(EnvelopeKeyManager::new()),
            signer: Box::new(ForbiddenSigner),
            timestamp_service: Box::new(Timestamp),
            signature_verifier: Box::new(RsaPkcs1Verifier),
            certificate_validator: Box::new(X509ChainValidator),
            timestamp_verifier: Box::new(Timestamp),
            archiver: Box::new(StoredZipWriter),
        },
        EvidenceMaterial {
            signer_certificate_pem: b"unused global cert".to_vec(),
            issuer_certificate_pem: b"unused global root".to_vec(),
            crl_pem: b"unused global crl".to_vec(),
            tsa_chain_pem: Some(b"synthetic TSA trust for atomicity tests".to_vec()),
            openssl_version: "synthetic timestamp adapter".into(),
        },
        Zeroizing::new(vec![7; 32]),
    )
    .unwrap()
}
