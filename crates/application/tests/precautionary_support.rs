#[path = "precautionary_support_helpers/mod.rs"]
mod support;

use application::documents::{StageDocumentFormat, StageFormatPolicy};
use application::ApplicationError;
use domain::crypto::{cipher::document_aad, DocumentId, DocumentVersion, Sha256Digest};
use support::*;
use uuid::Uuid;

#[test]
fn exact_pending_support_preserves_record_metadata_and_detected_format() {
    for format in [StageDocumentFormat::Pdf, StageDocumentFormat::Docx] {
        let record = record();
        let selected = values(&record);
        let before = record.clone();
        let before_values = selected.clone();
        let mut fixture = Fixture::new(&record);
        fixture.validator.formats = vec![format];
        let admitted = fixture
            .admit(&selected, std::slice::from_ref(&record))
            .unwrap();
        assert_eq!(admitted.reference, reference(&record));
        assert_eq!(admitted.digest, record.digest);
        assert_eq!(admitted.name, "scheduling-source.bin");
        assert_eq!(admitted.format, format);
        assert_eq!(admitted.policy, StageFormatPolicy::PdfDocxV1);
        assert_eq!(record, before);
        assert_eq!(selected, before_values);
        assert_eq!(fixture.events(), ["unwrap", "open", "hash", "parser"]);
        assert_eq!(fixture.validator.calls(), 1);
        assert_eq!(
            fixture.aad(),
            [document_aad(record.id, record.version).to_vec()]
        );
    }
}

#[test]
fn sealed_support_verifies_captured_evidence_before_one_format_admission() {
    let record = sealed_record();
    let before = record.clone();
    let fixture = Fixture::new(&record);
    let admitted = fixture
        .admit(&values(&record), std::slice::from_ref(&record))
        .unwrap();
    assert_eq!(admitted.reference, reference(&record));
    assert_eq!(admitted.digest, record.digest);
    assert_eq!(record, before);
    assert_eq!(
        fixture.events(),
        [
            "unwrap",
            "open",
            "hash",
            "signature",
            "timestamp",
            "certificate",
            "parser",
        ]
    );
    assert_eq!(fixture.validator.calls(), 1);
}

#[test]
fn missing_duplicate_and_excess_support_records_reject_before_crypto() {
    let record = record();
    let selected = values(&record);
    let mut unrelated = record.clone();
    unrelated.id = DocumentId::from_uuid(Uuid::from_u128(78));
    for records in [
        vec![],
        vec![record.clone(), record.clone()],
        vec![record.clone(), unrelated.clone()],
        vec![record.clone(), unrelated, record.clone()],
    ] {
        let fixture = Fixture::new(&record);
        assert!(matches!(
            fixture.admit(&selected, &records),
            Err(ApplicationError::InvalidInput(_))
        ));
        fixture.assert_no_crypto_or_parser();
    }
}

#[test]
fn exact_document_identity_and_version_are_required_before_crypto() {
    let record = record();
    let selected = values(&record);
    for change_id in [true, false] {
        let mut changed = record.clone();
        if change_id {
            changed.id = DocumentId::from_uuid(Uuid::from_u128(78));
        } else {
            changed.version = DocumentVersion::new(5).unwrap();
        }
        let fixture = Fixture::new(&record);
        assert!(matches!(
            fixture.admit(&selected, &[changed]),
            Err(ApplicationError::InvalidInput(_))
        ));
        fixture.assert_no_crypto_or_parser();
    }
}

#[test]
fn the_selected_digest_must_match_the_record_before_crypto() {
    let mut record = record();
    let selected = values(&record);
    record.digest = Sha256Digest::from_array([0x99; 32]);
    let fixture = Fixture::new(&record);
    assert!(matches!(
        fixture.admit(&selected, &[record]),
        Err(ApplicationError::InvalidInput(_))
    ));
    fixture.assert_no_crypto_or_parser();
}

#[test]
fn unsafe_or_oversized_record_names_reject_before_crypto() {
    let original = record();
    for name in [
        "".into(),
        "../source.pdf".into(),
        "source/path.pdf".into(),
        "source\n.pdf".into(),
        "x".repeat(129),
    ] {
        let mut record = original.clone();
        record.name = name;
        let fixture = Fixture::new(&record);
        assert!(fixture.admit(&values(&record), &[record]).is_err());
        fixture.assert_no_crypto_or_parser();
    }
}

#[test]
fn malformed_vault_and_oversized_evidence_reject_before_crypto() {
    let mut malformed = record();
    malformed.vault.truncate(5);
    let mut oversized = sealed_record();
    oversized.evidence.as_mut().unwrap().crl_pem = vec![b'c'; 1_048_577];
    for record in [malformed, oversized] {
        let fixture = Fixture::new(&record);
        assert!(fixture.admit(&values(&record), &[record]).is_err());
        fixture.assert_no_crypto_or_parser();
    }
}

#[test]
fn matching_selected_and_record_digests_still_require_plaintext_rehashing() {
    let mut record = record();
    record.digest = Sha256Digest::from_array([0x99; 32]);
    let selected = values(&record);
    let fixture = Fixture::new(&record);
    assert!(matches!(
        fixture.admit(&selected, &[record]),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert_eq!(fixture.events(), ["unwrap", "open", "hash"]);
    assert_eq!(fixture.validator.calls(), 0);
}

#[test]
fn changed_encrypted_content_cannot_reuse_the_selected_digest() {
    let mut record = record();
    let selected = values(&record);
    record.vault = record_with_content(b"Changed scheduling source").vault;
    let fixture = Fixture::new(&record);
    assert!(matches!(
        fixture.admit(&selected, &[record]),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert_eq!(fixture.events(), ["unwrap", "open", "hash"]);
    assert_eq!(fixture.validator.calls(), 0);
}

#[test]
fn invalid_captured_signature_timestamp_or_incomplete_evidence_prevents_admission() {
    for field in 0..3 {
        let mut record = sealed_record();
        let evidence = record.evidence.as_mut().unwrap();
        match field {
            0 => evidence.signature[0] ^= 1,
            1 => evidence.timestamp_token[0] ^= 1,
            _ => evidence.crl_pem.clear(),
        }
        let fixture = Fixture::new(&record);
        assert!(matches!(
            fixture.admit(&values(&record), &[record]),
            Err(ApplicationError::StoredDocumentInconsistent(_))
        ));
        assert_eq!(fixture.validator.calls(), 0);
        assert!(!fixture.events().contains(&"parser"));
    }
}

#[test]
fn format_rejection_limits_and_adapter_failure_are_propagated_without_retry() {
    for failure in [
        Failure::FormatRejected,
        Failure::ValidationLimit,
        Failure::Port,
    ] {
        let record = record();
        let mut fixture = Fixture::new(&record);
        fixture.validator.failure = Some(failure);
        let error = fixture.admit(&values(&record), &[record]).unwrap_err();
        match failure {
            Failure::FormatRejected => assert!(matches!(
                error,
                ApplicationError::StageSupportFormatRejected
            )),
            Failure::ValidationLimit => assert!(matches!(
                error,
                ApplicationError::StageSupportValidationLimit
            )),
            Failure::Port => assert!(
                matches!(error, ApplicationError::Port(message) if message == "format adapter unavailable")
            ),
        }
        assert_eq!(fixture.events(), ["unwrap", "open", "hash", "parser"]);
        assert_eq!(fixture.validator.calls(), 1);
    }
}

#[test]
fn format_result_must_describe_exactly_the_one_admitted_record() {
    for formats in [
        vec![],
        vec![StageDocumentFormat::Pdf, StageDocumentFormat::Docx],
    ] {
        let record = record();
        let mut fixture = Fixture::new(&record);
        fixture.validator.formats = formats;
        assert!(matches!(
            fixture.admit(&values(&record), &[record]),
            Err(ApplicationError::Port(_))
        ));
        assert_eq!(fixture.events(), ["unwrap", "open", "hash", "parser"]);
        assert_eq!(fixture.validator.calls(), 1);
    }
}
