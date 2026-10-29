#[allow(dead_code)]
#[path = "support/individual_evidence.rs"]
mod proofs;

use application::documents::{DocumentProcessor, DocumentRecord};
use domain::{
    crypto::{Sha256Digest, Signature, SignatureVerification, SignatureVerifier},
    DomainError,
};
use zeroize::Zeroizing;

struct ContentSignature;
impl SignatureVerifier for ContentSignature {
    fn verify(
        &self,
        _: &Sha256Digest,
        _: &Signature,
        _: &[u8],
    ) -> Result<SignatureVerification, DomainError> {
        Ok(SignatureVerification::Valid)
    }
}
fn input() -> DocumentRecord {
    let (plain, mut seal, proof) = proofs::fixture();
    let mut record = super::crypto::processor()
        .prepare_version(plain.id, plain.version, &plain.name, b"PDF fixture")
        .unwrap();
    assert_eq!(record.digest, plain.digest);
    seal.timestamp_token = record.digest.as_bytes().to_vec();
    seal.individual = Some(proof);
    record.seal(seal).unwrap();
    record
}
fn processor(credential: bool) -> DocumentProcessor {
    let mut ports = super::crypto::processor_ports();
    ports.signature_verifier = Box::new(ContentSignature);
    let reader = DocumentProcessor::new(
        ports,
        super::crypto::evidence_material(),
        Zeroizing::new(vec![0x44; 32]),
    )
    .unwrap();
    if credential {
        reader.with_individual_verifier(Box::new(proofs::Verifier::default()))
    } else {
        reader
    }
}
#[test]
fn personal_proofs_are_checked_for_verification_export_and_support_validation() {
    let record = input();
    let reader = processor(true);
    reader.validate_record(&record).unwrap();
    reader.verify(&record, 300).unwrap();
    reader.export_evidence(&record).unwrap();
    for field in 0..2 {
        let mut altered = record.clone();
        let proof = altered
            .evidence
            .as_mut()
            .unwrap()
            .individual
            .as_mut()
            .unwrap();
        if field == 0 {
            proof.binding_signature[0] ^= 1;
        } else {
            proof.consent_statement[200] ^= 1;
        }
        assert!(reader.validate_record(&altered).is_err());
        assert!(reader.verify(&altered, 300).is_err());
        assert!(reader.export_evidence(&altered).is_err());
    }
}
#[test]
fn personal_evidence_cannot_silently_pass_a_legacy_only_processor() {
    let reader = processor(false);
    let record = input();
    assert!(reader.validate_record(&record).is_err());
    assert!(reader.verify(&record, 300).is_err());
    assert!(reader.export_evidence(&record).is_err());
}

#[test]
fn personal_certificate_is_evaluated_at_acceptance_not_content_timestamp() {
    use domain::crypto::{CertificateSummary, CertificateValidation, CertificateValidator};
    struct AcceptanceTime;
    impl CertificateValidator for AcceptanceTime {
        fn validate(
            &self,
            _: &[u8],
            _: &[u8],
            _: Option<&[u8]>,
            at: i64,
        ) -> Result<CertificateValidation, DomainError> {
            assert_eq!(
                at, 300,
                "individual acceptance is independent of the content timestamp"
            );
            Ok(CertificateValidation::Valid)
        }
        fn inspect(&self, _: &[u8]) -> Result<CertificateSummary, DomainError> {
            unreachable!("record validation does not inspect display metadata")
        }
    }
    let mut ports = super::crypto::processor_ports();
    ports.signature_verifier = Box::new(ContentSignature);
    ports.certificate_validator = Box::new(AcceptanceTime);
    let reader = DocumentProcessor::new(
        ports,
        super::crypto::evidence_material(),
        Zeroizing::new(vec![0x44; 32]),
    )
    .unwrap()
    .with_individual_verifier(Box::new(proofs::Verifier::default()));
    reader.validate_record(&input()).unwrap();
}

#[test]
fn personal_verification_judges_acceptance_and_reports_today_only_as_a_note() {
    use application::verification::{ComponentStatus, Verdict};
    use domain::crypto::{CertificateSummary, CertificateValidation, CertificateValidator};
    struct ListExpiredToday;
    impl CertificateValidator for ListExpiredToday {
        fn validate(
            &self,
            _: &[u8],
            _: &[u8],
            crl: Option<&[u8]>,
            at: i64,
        ) -> Result<CertificateValidation, DomainError> {
            match (at, crl.is_some()) {
                (300, true) | (10_000, false) => Ok(CertificateValidation::Valid),
                (10_000, true) => Err(DomainError::StaleCrl {
                    next_update_unix: 5_000,
                }),
                other => panic!("unexpected evaluation {other:?}"),
            }
        }
        fn inspect(&self, _: &[u8]) -> Result<CertificateSummary, DomainError> {
            unreachable!("verification does not inspect display metadata")
        }
    }
    let mut ports = super::crypto::processor_ports();
    ports.signature_verifier = Box::new(ContentSignature);
    ports.certificate_validator = Box::new(ListExpiredToday);
    let reader = DocumentProcessor::new(
        ports,
        super::crypto::evidence_material(),
        Zeroizing::new(vec![0x44; 32]),
    )
    .unwrap()
    .with_individual_verifier(Box::new(proofs::Verifier::default()));
    let report = reader.verify(&input(), 10_000).unwrap();
    assert_eq!(report.certificate.status, ComponentStatus::Passed);
    assert!(
        report
            .certificate
            .detail
            .contains("signing time 1970-01-01T00:05:00Z"),
        "{}",
        report.certificate.detail
    );
    assert_eq!(report.verdict, Verdict::Valid);
}
