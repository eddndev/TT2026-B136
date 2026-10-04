#[allow(dead_code)]
#[path = "precautionary_support_helpers/mod.rs"]
mod support;

use application::{
    documents::{DocumentRecord, StageDocumentFormat, StageFormatPolicy, StageSupportReadLimits},
    precautionary_measures::admit_measure_decision_support,
    ApplicationError,
};
use domain::{
    crypto::{DocumentId, DocumentVersion, Sha256Digest},
    hearings::{HearingNote, HearingSupportRef},
    precautionary_measures::{MeasureDecisionValues, MeasureDecisionValuesInput, MeasureTime},
};
use support::*;
use uuid::Uuid;

fn declaration(record: &DocumentRecord) -> MeasureDecisionValues {
    MeasureDecisionValues::new(MeasureDecisionValuesInput {
        authority: HearingNote::new("Declared court").unwrap(),
        declared_at: MeasureTime::new(
            domain::procedural_time::DeclaredProceduralTime::unknown(),
            Some(HearingNote::new("Time not stated").unwrap()),
        )
        .unwrap(),
        justification: HearingNote::new("Declared judicial decision").unwrap(),
        support: HearingSupportRef::new(reference(record), record.digest),
        locator: HearingNote::new("Page 1").unwrap(),
    })
}

#[test]
fn decision_support_uses_its_exact_selection_before_any_crypto() {
    let original = record();
    let values = declaration(&original);
    for mutation in 0..5 {
        let mut candidate = original.clone();
        match mutation {
            0 => candidate.id = DocumentId::from_uuid(Uuid::from_u128(88)),
            1 => candidate.version = DocumentVersion::initial(),
            2 => candidate.digest = Sha256Digest::from_array([99; 32]),
            _ => {}
        }
        let records = match mutation {
            3 => vec![],
            4 => vec![candidate.clone(), candidate],
            _ => vec![candidate],
        };
        let fixture = Fixture::new(&original);
        assert!(admit_measure_decision_support(
            &values,
            &records,
            fixture.processor(),
            &StageSupportReadLimits::standard(),
            &fixture.validator,
        )
        .is_err());
        fixture.assert_no_crypto_or_parser();
    }
}

#[test]
fn declared_decision_support_preserves_exact_metadata_and_detected_format() {
    for record in [record(), sealed_record()] {
        let values = declaration(&record);
        let before = record.clone();
        let mut fixture = Fixture::new(&record);
        fixture.validator.formats = vec![StageDocumentFormat::Docx];
        let admitted = admit_measure_decision_support(
            &values,
            std::slice::from_ref(&record),
            fixture.processor(),
            &StageSupportReadLimits::standard(),
            &fixture.validator,
        )
        .unwrap();
        assert_eq!(admitted.reference, reference(&record));
        assert_eq!(admitted.digest, record.digest);
        assert_eq!(admitted.name, record.name);
        assert_eq!(admitted.format, StageDocumentFormat::Docx);
        assert_eq!(admitted.policy, StageFormatPolicy::PdfDocxV1);
        assert_eq!(record, before);
        assert_eq!(fixture.validator.calls(), 1);
        let expected = if record.evidence.is_some() {
            vec![
                "unwrap",
                "open",
                "hash",
                "signature",
                "timestamp",
                "certificate",
                "parser",
            ]
        } else {
            vec!["unwrap", "open", "hash", "parser"]
        };
        assert_eq!(fixture.events(), expected);
    }
}

#[test]
fn matching_decision_digests_do_not_bypass_plaintext_integrity() {
    let mut record = record();
    record.digest = Sha256Digest::from_array([99; 32]);
    let fixture = Fixture::new(&record);
    assert!(matches!(
        admit_measure_decision_support(
            &declaration(&record),
            &[record],
            fixture.processor(),
            &StageSupportReadLimits::standard(),
            &fixture.validator,
        ),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert_eq!(fixture.events(), ["unwrap", "open", "hash"]);
    assert_eq!(fixture.validator.calls(), 0);
}

#[test]
fn decision_parser_rejection_is_propagated_without_retry() {
    let record = record();
    let mut fixture = Fixture::new(&record);
    fixture.validator.failure = Some(Failure::FormatRejected);
    assert!(matches!(
        admit_measure_decision_support(
            &declaration(&record),
            &[record],
            fixture.processor(),
            &StageSupportReadLimits::standard(),
            &fixture.validator,
        ),
        Err(ApplicationError::StageSupportFormatRejected)
    ));
    assert_eq!(fixture.validator.calls(), 1);
}
