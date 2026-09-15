#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod document_format_support;

use application::{
    documents::{
        DocumentFormatBatch, DocumentFormatBatchValidator, StageDocumentFormat,
        StageSupportReadLimits,
    },
    ApplicationError,
};
use document_format_support::Observations;
use domain::crypto::{cipher::document_aad, DocumentVersion};
use std::sync::{Arc, Mutex};

struct BorrowCheck(Arc<Mutex<Observations>>);
impl DocumentFormatBatchValidator for BorrowCheck {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        let mut observations = self.0.lock().unwrap();
        assert_eq!(batch.inputs().len(), observations.pointers.len());
        for (input, pointer) in batch.inputs().iter().zip(&observations.pointers) {
            assert_eq!(
                input.bytes().as_ptr() as usize,
                *pointer,
                "the cipher output must be borrowed without a plaintext clone"
            );
        }
        observations.events.push("parser");
        Ok(vec![StageDocumentFormat::Pdf; batch.inputs().len()])
    }
}

#[test]
fn each_sealed_version_is_validated_once_with_its_aad_and_without_plaintext_clones() {
    let writer = crypto::processor();
    let first = writer.prepare("original.pdf", b"first content").unwrap();
    let second = writer
        .prepare_version(
            first.id,
            DocumentVersion::new(2).unwrap(),
            "amended.pdf",
            b"second content",
        )
        .unwrap();
    let records = [writer.seal(&first).unwrap(), writer.seal(&second).unwrap()];
    let (reader, observed) = document_format_support::processor();
    reader
        .validate_support_batch(
            &records,
            &StageSupportReadLimits::standard(),
            &BorrowCheck(observed.clone()),
        )
        .unwrap();
    let observed = observed.lock().unwrap();
    assert_eq!(
        observed.events,
        [
            "unwrap",
            "open",
            "hash",
            "signature",
            "timestamp",
            "certificate",
            "unwrap",
            "open",
            "hash",
            "signature",
            "timestamp",
            "certificate",
            "parser"
        ]
    );
    assert_eq!(
        observed.aad,
        records
            .iter()
            .map(|r| document_aad(r.id, r.version).to_vec())
            .collect::<Vec<_>>()
    );
}

#[test]
fn all_records_are_admitted_before_the_first_crypto_operation() {
    let writer = crypto::processor();
    let first = writer.prepare("first.pdf", b"first").unwrap();
    let second = writer.prepare("second.pdf", b"second").unwrap();
    for field in 0..4 {
        let mut oversized = second.clone();
        match field {
            0 => oversized.vault.resize(16_777_314, 0),
            1 => oversized.vault[5..9].copy_from_slice(&59u32.to_be_bytes()),
            2 => oversized.name = "a".repeat(129),
            _ => {
                oversized = writer.seal(&oversized).unwrap();
                oversized.evidence.as_mut().unwrap().openssl_version = "a".repeat(1_048_576);
            }
        }
        let (reader, observed) = document_format_support::processor();
        assert!(reader
            .validate_support_batch(
                &[first.clone(), oversized],
                &StageSupportReadLimits::standard(),
                &BorrowCheck(observed.clone())
            )
            .is_err());
        assert!(observed.lock().unwrap().events.is_empty());
    }
}

#[test]
fn the_two_document_plaintext_boundary_is_admitted_in_one_batch() {
    let writer = crypto::processor();
    let bytes = vec![42; 16 * 1024 * 1024];
    let first = writer.prepare("first.pdf", &bytes).unwrap();
    let second = writer.prepare("second.pdf", &bytes).unwrap();
    drop(bytes);
    let (reader, observed) = document_format_support::processor();
    let results = reader
        .validate_support_batch(
            &[first, second],
            &StageSupportReadLimits::standard(),
            &BorrowCheck(observed.clone()),
        )
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(
        observed.lock().unwrap().events,
        ["unwrap", "open", "hash", "unwrap", "open", "hash", "parser"]
    );
}

#[test]
fn format_rejections_and_resource_limits_do_not_trigger_a_second_attempt() {
    struct Rejected(bool, Arc<Mutex<Observations>>);
    impl DocumentFormatBatchValidator for Rejected {
        fn validate_batch(
            &self,
            _: &DocumentFormatBatch<'_>,
        ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
            self.1.lock().unwrap().events.push("parser");
            Err(if self.0 {
                ApplicationError::StageSupportFormatRejected
            } else {
                ApplicationError::StageSupportValidationLimit
            })
        }
    }
    let record = crypto::processor().prepare("file.pdf", b"content").unwrap();
    for format_error in [true, false] {
        let (reader, observed) = document_format_support::processor();
        let error = reader
            .validate_support_batch(
                std::slice::from_ref(&record),
                &StageSupportReadLimits::standard(),
                &Rejected(format_error, observed.clone()),
            )
            .unwrap_err();
        if format_error {
            assert!(matches!(
                error,
                ApplicationError::StageSupportFormatRejected
            ));
        } else {
            assert!(matches!(
                error,
                ApplicationError::StageSupportValidationLimit
            ));
        }
        assert_eq!(
            observed.lock().unwrap().events,
            ["unwrap", "open", "hash", "parser"]
        );
    }
}

#[test]
fn a_cipher_result_with_a_length_different_from_its_envelope_is_rejected() {
    use domain::{
        crypto::{AuthenticatedCipher, SealedPayload},
        DomainError,
    };
    struct WrongLength;
    impl AuthenticatedCipher for WrongLength {
        fn seal(&self, _: &[u8], _: &[u8], _: &[u8]) -> Result<SealedPayload, DomainError> {
            panic!("reader must not encrypt")
        }
        fn open(&self, _: &[u8], _: &[u8], _: &SealedPayload) -> Result<Vec<u8>, DomainError> {
            Ok(b"content".to_vec())
        }
    }
    let mut record = crypto::processor().prepare("file.pdf", b"content").unwrap();
    record.vault.extend_from_slice(&[0; 10]);
    let mut ports = crypto::processor_ports();
    ports.cipher = Box::new(WrongLength);
    let reader = application::documents::DocumentProcessor::new(
        ports,
        crypto::evidence_material(),
        zeroize::Zeroizing::new(vec![1; 32]),
    )
    .unwrap();
    let observed = Arc::new(Mutex::new(Observations::default()));
    assert!(matches!(
        reader.validate_support_batch(
            &[record],
            &StageSupportReadLimits::standard(),
            &BorrowCheck(observed.clone())
        ),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert!(observed.lock().unwrap().events.is_empty());
}
