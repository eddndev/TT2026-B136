use application::{
    documents::{
        DocumentFormatBatch, DocumentFormatBatchValidator, DocumentProcessor,
        DocumentProcessorPorts, EvidenceMaterial, StageDocumentFormat,
    },
    ApplicationError,
};
use domain::{
    crypto::{DocumentSigner, Sha256Digest, Signature},
    DomainError,
};
use infrastructure::{
    EnvelopeKeyManager, LocalOpensslTsa, ManifestTimestampAuthority, Rfc3161Verifier,
    RingAesGcmCipher, RingSha256Hasher, RsaPkcs1Verifier, StoredZipWriter, X509ChainValidator,
};
use zeroize::Zeroizing;

struct ForbiddenSigner;
impl DocumentSigner for ForbiddenSigner {
    fn sign(&self, _: &Sha256Digest) -> Result<Signature, DomainError> {
        panic!("no global signing")
    }
}
/// The time-stamping authority issued by the declaration fixture root.
pub fn authority() -> LocalOpensslTsa {
    LocalOpensslTsa::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki/tsa.cnf"),
        &crate::declaration_fixture::fixture().tsa,
    )
}
/// The manifest time-stamping port over the same authority.
pub fn manifest_timestamps() -> ManifestTimestampAuthority {
    ManifestTimestampAuthority::new(Box::new(authority()), Box::new(Rfc3161Verifier::new()))
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
            timestamp_service: Box::new(authority()),
            signature_verifier: Box::new(RsaPkcs1Verifier),
            certificate_validator: Box::new(X509ChainValidator),
            timestamp_verifier: Box::new(Rfc3161Verifier::new()),
            archiver: Box::new(StoredZipWriter),
        },
        EvidenceMaterial {
            signer_certificate_pem: b"unused global cert".to_vec(),
            issuer_certificate_pem: b"unused global root".to_vec(),
            crl_pem: b"unused global crl".to_vec(),
            tsa_chain_pem: Some(crate::declaration_fixture::fixture().root.clone()),
            openssl_version: "local OpenSSL time-stamping authority".into(),
        },
        Zeroizing::new(vec![7; 32]),
    )
    .unwrap()
}
