#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;

use application::documents::{
    DocumentFormatBatch, DocumentFormatBatchValidator, DocumentRecord, StageDocumentFormat,
    StageFormatPolicy, StageSupportReadLimits,
};
use application::ApplicationError;
use std::sync::atomic::{AtomicUsize, Ordering};

struct InspectBatch {
    expected: Vec<(application::documents::DocumentVersionRef, Vec<u8>)>,
    outputs: Vec<StageDocumentFormat>,
    calls: AtomicUsize,
}

impl DocumentFormatBatchValidator for InspectBatch {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(batch.inputs().len(), self.expected.len());
        assert_eq!(
            batch.total_bytes(),
            self.expected
                .iter()
                .map(|(_, bytes)| bytes.len())
                .sum::<usize>()
        );
        for (input, (reference, bytes)) in batch.inputs().iter().zip(&self.expected) {
            assert_eq!(input.reference(), *reference);
            assert_eq!(input.bytes(), bytes);
        }
        Ok(self.outputs.clone())
    }
}

fn validator(records: &[DocumentRecord], contents: &[&[u8]]) -> InspectBatch {
    InspectBatch {
        expected: records
            .iter()
            .zip(contents)
            .map(|(record, bytes)| {
                (
                    application::documents::DocumentVersionRef {
                        id: record.id,
                        version: record.version,
                    },
                    bytes.to_vec(),
                )
            })
            .collect(),
        outputs: vec![StageDocumentFormat::Pdf; records.len()],
        calls: AtomicUsize::new(0),
    }
}

#[test]
fn two_supports_are_crypto_validated_and_lent_in_order_to_one_batch() {
    let processor = crypto::processor();
    let first = processor.prepare("first.pdf", b"first plaintext").unwrap();
    let second = processor
        .prepare("second.docx", b"second plaintext")
        .unwrap();
    let records = [processor.seal(&first).unwrap(), second];
    let before = records.clone();
    let mut check = validator(&records, &[b"first plaintext", b"second plaintext"]);
    check.outputs = vec![StageDocumentFormat::Pdf, StageDocumentFormat::Docx];
    assert_eq!(
        processor
            .validate_support_batch(&records, &StageSupportReadLimits::standard(), &check)
            .unwrap(),
        check.outputs
    );
    assert_eq!(check.calls.load(Ordering::SeqCst), 1);
    assert_eq!(records, before);
    assert_eq!(StageFormatPolicy::PdfDocxV1.as_str(), "pdf_docx_v1");
}

#[test]
fn empty_excessive_and_duplicate_batches_never_reach_the_parser() {
    let processor = crypto::processor();
    let first = processor.prepare("first.pdf", b"first").unwrap();
    let records = [first.clone(), first.clone(), first];
    let check = validator(&[], &[]);
    for invalid in [&records[..0], &records[..2], &records[..3]] {
        assert!(matches!(
            processor.validate_support_batch(invalid, &StageSupportReadLimits::standard(), &check),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    assert_eq!(check.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn incorrect_parser_result_cardinality_is_a_port_failure() {
    let processor = crypto::processor();
    let records = [processor.prepare("first.pdf", b"first").unwrap()];
    for outputs in [
        vec![],
        vec![StageDocumentFormat::Pdf, StageDocumentFormat::Docx],
    ] {
        let mut check = validator(&records, &[b"first"]);
        check.outputs = outputs;
        assert!(matches!(
            processor.validate_support_batch(&records, &StageSupportReadLimits::standard(), &check),
            Err(ApplicationError::Port(_))
        ));
        assert_eq!(check.calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn a_second_support_with_changed_plaintext_prevents_the_entire_parser_batch() {
    let processor = crypto::processor();
    let first = processor.prepare("first.pdf", b"first").unwrap();
    let mut second = processor.prepare("second.pdf", b"second").unwrap();
    second.vault[81] ^= 1;
    let check = validator(&[], &[]);
    assert!(matches!(
        processor.validate_support_batch(
            &[first, second],
            &StageSupportReadLimits::standard(),
            &check
        ),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert_eq!(check.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn altered_signature_timestamp_or_incomplete_evidence_prevents_parser_execution() {
    let processor = crypto::processor();
    let pending = processor.prepare("first.pdf", b"first").unwrap();
    let sealed = processor.seal(&pending).unwrap();
    for field in 0..3 {
        let mut altered = sealed.clone();
        let evidence = altered.evidence.as_mut().unwrap();
        match field {
            0 => evidence.signature[0] ^= 1,
            1 => evidence.timestamp_token[0] ^= 1,
            _ => evidence.crl_pem.clear(),
        }
        let check = validator(&[], &[]);
        assert!(matches!(
            processor.validate_support_batch(
                &[altered],
                &StageSupportReadLimits::standard(),
                &check
            ),
            Err(ApplicationError::StoredDocumentInconsistent(_))
        ));
        assert_eq!(check.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn read_limits_bound_the_total_decoded_evidence_without_changing_legacy_reads() {
    let processor = crypto::processor();
    let pending = processor.prepare("first.pdf", b"first").unwrap();
    let mut sealed = processor.seal(&pending).unwrap();
    let limits = StageSupportReadLimits::standard();
    assert_eq!(limits.max_evidence_json_bytes(), 1024 * 1024);
    assert_eq!(limits.vault().max_vault_bytes(), 16_777_313);
    let evidence = sealed.evidence.as_mut().unwrap();
    evidence.crl_pem = vec![b'c'; 600_000];
    evidence.issuer_certificate_pem = vec![b'i'; 500_000];
    assert!(matches!(
        limits.check_record(&sealed),
        Err(ApplicationError::StageSupportTooLarge)
    ));
    processor.validate_record(&sealed).unwrap();
    let check = validator(&[], &[]);
    assert!(matches!(
        processor.validate_support_batch(&[sealed], &limits, &check),
        Err(ApplicationError::StageSupportTooLarge)
    ));
    assert_eq!(check.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn the_decoded_evidence_limit_includes_every_field_at_the_exact_boundary() {
    let processor = crypto::processor();
    let pending = processor.prepare("file.pdf", b"content").unwrap();
    let mut record = processor.seal(&pending).unwrap();
    let evidence = record.evidence.as_mut().unwrap();
    let fixed = evidence.signature.len()
        + evidence.timestamp_token.len()
        + evidence.signer_certificate_pem.len()
        + evidence.issuer_certificate_pem.len()
        + evidence.tsa_chain_pem.as_ref().unwrap().len()
        + evidence.openssl_version.len();
    evidence.crl_pem = vec![b'c'; 1024 * 1024 - fixed];
    let limits = StageSupportReadLimits::standard();
    assert_eq!(limits.check_record(&record).unwrap(), 7);
    for field in 0..7 {
        let mut excessive = record.clone();
        let evidence = excessive.evidence.as_mut().unwrap();
        match field {
            0 => evidence.signature.push(1),
            1 => evidence.timestamp_token.push(1),
            2 => evidence.signer_certificate_pem.push(1),
            3 => evidence.issuer_certificate_pem.push(1),
            4 => evidence.crl_pem.push(1),
            5 => evidence.tsa_chain_pem.as_mut().unwrap().push(1),
            _ => evidence.openssl_version.push('a'),
        }
        assert!(matches!(
            limits.check_record(&excessive),
            Err(ApplicationError::StageSupportTooLarge)
        ));
    }
}
