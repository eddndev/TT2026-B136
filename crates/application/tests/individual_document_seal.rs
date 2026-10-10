#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;

use application::documents::{DocumentProcessor, ExternalSignerMaterial};
use application::ApplicationError;
use domain::crypto::{DocumentSigner, Sha256Digest, Signature, TimestampService};
use domain::DomainError;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use zeroize::Zeroizing;

struct ForbiddenSigner;
impl DocumentSigner for ForbiddenSigner {
    fn sign(&self, _: &Sha256Digest) -> Result<Signature, DomainError> {
        panic!("individual signing must never use the server private key")
    }
}
struct Timestamp(Arc<AtomicUsize>, bool);
impl TimestampService for Timestamp {
    fn request(&self, digest: &Sha256Digest) -> Result<Vec<u8>, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(if self.1 {
            vec![0; 32]
        } else {
            digest.as_bytes().to_vec()
        })
    }
}
fn reader(invalid_timestamp: bool) -> (DocumentProcessor, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut ports = crypto::processor_ports();
    ports.signer = Box::new(ForbiddenSigner);
    ports.timestamp_service = Box::new(Timestamp(calls.clone(), invalid_timestamp));
    (
        DocumentProcessor::new(
            ports,
            crypto::evidence_material(),
            Zeroizing::new(vec![0x44; 32]),
        )
        .unwrap(),
        calls,
    )
}
fn material() -> ExternalSignerMaterial {
    ExternalSignerMaterial {
        certificate_pem: b"personal certificate".to_vec(),
        issuer_pem: b"personal issuer".to_vec(),
        crl_pem: b"personal crl".to_vec(),
    }
}
#[test]
fn external_signature_retains_the_personal_material_and_configured_tsa() {
    let (processor, calls) = reader(false);
    let pending = processor
        .prepare("reviewed.pdf", b"reviewed exact PDF bytes")
        .unwrap();
    let signature = Signature::from_bytes(pending.digest.as_bytes().to_vec()).unwrap();
    let sealed = processor
        .seal_external(&pending, &signature, &material())
        .unwrap();
    assert_eq!(
        (sealed.id, sealed.version, sealed.digest),
        (pending.id, pending.version, pending.digest)
    );
    assert_eq!(sealed.vault, pending.vault);
    let evidence = sealed.evidence.unwrap();
    assert_eq!(evidence.signature, signature.as_bytes());
    assert_eq!(evidence.signer_certificate_pem, material().certificate_pem);
    assert_eq!(evidence.issuer_certificate_pem, material().issuer_pem);
    assert_eq!(evidence.crl_pem, material().crl_pem);
    assert_eq!(
        evidence.tsa_chain_pem,
        crypto::evidence_material().tsa_chain_pem
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(pending.evidence.is_none());
}
#[test]
fn mismatched_pdf_signature_never_requests_a_timestamp() {
    let (processor, calls) = reader(false);
    let pending = processor.prepare("reviewed.pdf", b"original").unwrap();
    let signature = Signature::from_bytes(vec![9; 32]).unwrap();
    assert!(processor
        .seal_external(&pending, &signature, &material())
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(pending.evidence.is_none());
}
#[test]
fn rejected_timestamp_cannot_produce_a_complete_record_or_retry() {
    let (processor, calls) = reader(true);
    let pending = processor.prepare("reviewed.pdf", b"original").unwrap();
    let signature = Signature::from_bytes(pending.digest.as_bytes().to_vec()).unwrap();
    assert!(matches!(
        processor.seal_external(&pending, &signature, &material()),
        Err(ApplicationError::TimestampEvidenceRejected(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(pending.evidence.is_none());
}
#[test]
fn historical_seal_cannot_be_replaced_by_a_personal_signature() {
    let writer = crypto::processor();
    let pending = writer.prepare("reviewed.pdf", b"original").unwrap();
    let sealed = writer.seal(&pending).unwrap();
    let (processor, calls) = reader(false);
    let signature = Signature::from_bytes(pending.digest.as_bytes().to_vec()).unwrap();
    assert!(matches!(
        processor.seal_external(&sealed, &signature, &material()),
        Err(ApplicationError::DocumentAlreadySealed(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[test]
fn changed_content_and_incomplete_certificate_material_fail_before_timestamp() {
    let (processor, calls) = reader(false);
    let pending = processor.prepare("reviewed.pdf", b"original").unwrap();
    let signature = Signature::from_bytes(pending.digest.as_bytes().to_vec()).unwrap();
    let mut changed = pending.clone();
    changed.digest = Sha256Digest::from_array([1; 32]);
    assert!(processor
        .seal_external(&changed, &signature, &material())
        .is_err());
    for field in 0..3 {
        let mut incomplete = material();
        match field {
            0 => incomplete.certificate_pem.clear(),
            1 => incomplete.issuer_pem.clear(),
            _ => incomplete.crl_pem.clear(),
        }
        assert!(processor
            .seal_external(&pending, &signature, &incomplete)
            .is_err());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[path = "individual_document_validation.rs"]
mod individual_document_validation;
