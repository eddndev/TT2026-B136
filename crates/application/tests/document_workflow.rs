#[path = "support/document_workflow.rs"]
mod support;

use application::documents::DocumentWorkflow;
use application::ApplicationError;
use domain::audit::ChainVerification;
use domain::crypto::DocumentId;

#[test]
fn upload_seal_verify_export_and_audit_form_one_workflow() {
    let workflow = support::workflow();

    let uploaded = workflow
        .upload("ana", "acta.txt", b"case document")
        .unwrap();
    assert_eq!(uploaded.version.get(), 1);
    assert_eq!(uploaded.name, "acta.txt");
    assert!(!uploaded.sealed);

    let sealed = workflow.seal("ana", uploaded.id).unwrap();
    assert!(sealed.sealed);

    let report = workflow.verify("ana", uploaded.id).unwrap();
    assert_eq!(report.verdict.to_string(), "valid");

    let package = workflow.export_evidence("ana", uploaded.id).unwrap();
    assert_eq!(package.archive, b"archive bytes");
    assert_eq!(package.file_name, "acta.txt-evidence.zip");

    assert_eq!(
        workflow.verify_audit().unwrap(),
        ChainVerification::Valid { entries: 4 }
    );
}

#[test]
fn an_unsealed_document_cannot_be_verified_or_exported() {
    let workflow = support::workflow();
    let uploaded = workflow
        .upload("ana", "acta.txt", b"case document")
        .unwrap();

    assert!(matches!(
        workflow.verify("ana", uploaded.id),
        Err(ApplicationError::DocumentNotSealed(_))
    ));
    assert!(matches!(
        workflow.export_evidence("ana", uploaded.id),
        Err(ApplicationError::DocumentNotSealed(_))
    ));
}

#[test]
fn a_missing_document_is_reported_consistently() {
    let workflow = support::workflow();

    assert!(matches!(
        workflow.seal("ana", DocumentId::new()),
        Err(ApplicationError::DocumentNotFound(_))
    ));
}

#[test]
fn a_document_cannot_be_sealed_twice() {
    let workflow = support::workflow();
    let uploaded = workflow
        .upload("ana", "acta.txt", b"case document")
        .unwrap();
    workflow.seal("ana", uploaded.id).unwrap();

    assert!(matches!(
        workflow.seal("ana", uploaded.id),
        Err(ApplicationError::DocumentAlreadySealed(_))
    ));
}

#[test]
fn historical_validation_preserves_bytes_and_rejects_corrupted_evidence() {
    let processor = support::processor();
    let pending = processor.prepare("acta.txt", b"case document").unwrap();
    processor.validate_record(&pending).unwrap();
    let sealed = processor.seal(&pending).unwrap();
    let before = sealed.clone();
    processor.validate_record(&sealed).unwrap();
    assert_eq!(sealed, before);

    let mut altered = sealed.clone();
    altered.digest = domain::crypto::Sha256Digest::from_array([0; 32]);
    assert!(processor.validate_record(&altered).is_err());
    let mut altered = sealed.clone();
    altered.evidence.as_mut().unwrap().signature[0] ^= 1;
    assert!(processor.validate_record(&altered).is_err());
    let mut altered = sealed;
    altered.evidence.as_mut().unwrap().timestamp_token[0] ^= 1;
    assert!(processor.validate_record(&altered).is_err());
}

#[test]
fn historical_validation_never_calls_signing_or_timestamp_creation() {
    struct NoSigner;
    impl domain::crypto::DocumentSigner for NoSigner {
        fn sign(
            &self,
            _: &domain::crypto::Sha256Digest,
        ) -> Result<domain::crypto::Signature, domain::DomainError> {
            panic!("historical validation must never sign")
        }
    }
    struct NoTimestamp;
    impl domain::crypto::TimestampService for NoTimestamp {
        fn request(
            &self,
            _: &domain::crypto::Sha256Digest,
        ) -> Result<Vec<u8>, domain::DomainError> {
            panic!("historical validation must never request a timestamp")
        }
    }
    let processor = support::processor();
    let record = processor
        .seal(&processor.prepare("acta.txt", b"case document").unwrap())
        .unwrap();
    let mut ports = support::processor_ports();
    ports.signer = Box::new(NoSigner);
    ports.timestamp_service = Box::new(NoTimestamp);
    let reader = application::documents::DocumentProcessor::new(
        ports,
        support::evidence_material(),
        zeroize::Zeroizing::new(vec![0x44; 32]),
    )
    .unwrap();
    reader.validate_record(&record).unwrap();
}

mod verification_mocks;

fn historical_reader(
    validator: verification_mocks::MockCertValidator,
    timestamp_verifier: Option<verification_mocks::MockTsVerifier>,
) -> application::documents::DocumentProcessor {
    let mut ports = support::processor_ports();
    ports.certificate_validator = Box::new(validator);
    if let Some(verifier) = timestamp_verifier {
        ports.timestamp_verifier = Box::new(verifier);
    }
    application::documents::DocumentProcessor::new(
        ports,
        support::evidence_material(),
        zeroize::Zeroizing::new(vec![0x44; 32]),
    )
    .unwrap()
}

fn sealed_record() -> application::documents::DocumentRecord {
    let processor = support::processor();
    processor
        .seal(&processor.prepare("acta.txt", b"case document").unwrap())
        .unwrap()
}

#[test]
fn historical_validation_rejects_malformed_issuer_and_crl_with_a_separate_tsa_chain() {
    for invalid_issuer in [true, false] {
        let mut record = sealed_record();
        let evidence = record.evidence.as_mut().unwrap();
        assert!(evidence.tsa_chain_pem.is_some());
        let expected_error = if invalid_issuer {
            evidence.issuer_certificate_pem = b"malformed issuer".to_vec();
            domain::DomainError::MalformedCertificate("invalid issuer".into())
        } else {
            evidence.crl_pem = b"malformed crl".to_vec();
            domain::DomainError::MalformedCrl("invalid crl".into())
        };
        let expected_evidence = evidence.clone();
        let expected_message = expected_error.to_string();
        let mut validator = verification_mocks::MockCertValidator::new();
        validator
            .expect_validate()
            .times(1)
            .withf(move |certificate, issuer, crl, _| {
                certificate == expected_evidence.signer_certificate_pem
                    && issuer == expected_evidence.issuer_certificate_pem
                    && *crl == Some(expected_evidence.crl_pem.as_slice())
            })
            .return_once(move |_, _, _, _| Err(expected_error));
        assert!(matches!(
            historical_reader(validator, None).validate_record(&record),
            Err(ApplicationError::Domain(error)) if error.to_string() == expected_message
        ));
    }
}

#[test]
fn historical_validation_evaluates_signer_material_at_the_captured_time_after_its_expiry() {
    use domain::crypto::{CertificateValidation, TimestampVerification};
    let record = sealed_record();
    let before = record.clone();
    let mut timestamp = verification_mocks::MockTsVerifier::new();
    timestamp.expect_verify().times(1).returning(|_, _, _| {
        Ok(TimestampVerification::Valid {
            generated_at: "2000-01-01T00:00:00Z".into(),
        })
    });
    let mut validator = verification_mocks::MockCertValidator::new();
    validator
        .expect_validate()
        .times(1)
        .returning(|_, _, crl, at| {
            assert!(
                crl.is_some(),
                "historical validation requires the captured crl"
            );
            assert_eq!(at, 946_684_800);
            Ok(if at <= 978_307_200 {
                CertificateValidation::Valid
            } else {
                CertificateValidation::Expired
            })
        });
    historical_reader(validator, Some(timestamp))
        .validate_record(&record)
        .unwrap();
    assert_eq!(record, before);
}

#[test]
fn historical_validation_rejects_every_unaccepted_certificate_status() {
    use domain::crypto::CertificateValidation;
    for outcome in [
        CertificateValidation::Expired,
        CertificateValidation::NotYetValid,
        CertificateValidation::UntrustedIssuer,
        CertificateValidation::Revoked {
            serial_hex: "1234".into(),
        },
    ] {
        let validator = verification_mocks::validator_reporting(outcome);
        assert!(matches!(
            historical_reader(validator, None).validate_record(&sealed_record()),
            Err(ApplicationError::StoredDocumentInconsistent(_))
        ));
    }
}

#[test]
fn historical_validation_rejects_an_unparseable_verified_timestamp_date() {
    let timestamp = verification_mocks::token_verifier_reporting(
        domain::crypto::TimestampVerification::Valid {
            generated_at: "unknown time".into(),
        },
    );
    assert!(matches!(
        historical_reader(
            verification_mocks::MockCertValidator::new(),
            Some(timestamp)
        )
        .validate_record(&sealed_record()),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
}
