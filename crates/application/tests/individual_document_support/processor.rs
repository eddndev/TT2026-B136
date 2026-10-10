use super::{pdf_signature, State, PDF};
use application::{
    documents::{
        DocumentFormatBatch, DocumentFormatBatchValidator, DocumentProcessor, StageDocumentFormat,
    },
    ApplicationError,
};
use domain::{
    crypto::{
        DocumentSigner, Sha256Digest, Signature, SignatureVerification, SignatureVerifier,
        TimestampService,
    },
    DomainError,
};
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

struct ForbiddenSigner;
impl DocumentSigner for ForbiddenSigner {
    fn sign(&self, _: &Sha256Digest) -> Result<Signature, DomainError> {
        panic!("individual document signing must never use a server private key")
    }
}
struct PdfVerifier;
impl SignatureVerifier for PdfVerifier {
    fn verify(
        &self,
        _: &Sha256Digest,
        signature: &Signature,
        certificate: &[u8],
    ) -> Result<SignatureVerification, DomainError> {
        assert_eq!(signature, &pdf_signature());
        assert!(certificate.starts_with(b"-----BEGIN CERTIFICATE-----"));
        Ok(SignatureVerification::Valid)
    }
}
struct Timestamp(Arc<Mutex<State>>);
impl TimestampService for Timestamp {
    fn request(&self, digest: &Sha256Digest) -> Result<Vec<u8>, DomainError> {
        let mut state = self.0.lock().unwrap();
        state.tsa_calls += 1;
        if state.timeout {
            return Err(DomainError::TimestampAuthorityFailure(
                "synthetic timeout".into(),
            ));
        }
        Ok(if state.bad_timestamp {
            vec![0; 32]
        } else {
            digest.as_bytes().to_vec()
        })
    }
}
pub struct Format(pub Arc<Mutex<State>>);
impl DocumentFormatBatchValidator for Format {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        assert_eq!(batch.inputs().len(), 1);
        assert_eq!(batch.inputs()[0].bytes(), PDF);
        let mut state = self.0.lock().unwrap();
        state.format_calls += 1;
        Ok(vec![if state.docx {
            StageDocumentFormat::Docx
        } else {
            StageDocumentFormat::Pdf
        }])
    }
}
pub fn processor(state: Arc<Mutex<State>>) -> DocumentProcessor {
    let mut ports = crate::crypto::processor_ports();
    ports.signer = Box::new(ForbiddenSigner);
    ports.signature_verifier = Box::new(PdfVerifier);
    ports.timestamp_service = Box::new(Timestamp(state));
    DocumentProcessor::new(
        ports,
        crate::crypto::evidence_material(),
        Zeroizing::new(vec![0x44; 32]),
    )
    .unwrap()
}
