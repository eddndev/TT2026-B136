#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[path = "../document_format_support/mod.rs"]
mod observed_crypto;

use application::case_stages::StageSupportSnapshot;
use application::documents::{
    DocumentFormatBatch, DocumentFormatBatchValidator, DocumentProcessor, DocumentRecord,
    StageDocumentFormat, StageSupportReadLimits,
};
use application::precautionary_hearings::admit_precautionary_support;
use application::ApplicationError;
use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef};
use domain::hearings::{
    HearingModality, HearingNote, HearingSupportRef, HearingTime, HearingVenue,
};
use domain::precautionary_hearings::{
    PrecautionaryHearingPurpose, PrecautionaryHearingSchedulingBasis, PrecautionaryHearingValues,
    PrecautionaryHearingValuesInput,
};
use observed_crypto::Observations;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use uuid::Uuid;

pub const CONTENT: &[u8] = b"Declared scheduling source";

pub fn record_with_content(content: &[u8]) -> DocumentRecord {
    crypto::processor()
        .prepare_version(
            DocumentId::from_uuid(Uuid::from_u128(77)),
            DocumentVersion::new(4).unwrap(),
            "scheduling-source.bin",
            content,
        )
        .unwrap()
}

pub fn record() -> DocumentRecord {
    record_with_content(CONTENT)
}

pub fn sealed_record() -> DocumentRecord {
    crypto::processor().seal(&record()).unwrap()
}

pub fn reference(record: &DocumentRecord) -> DocumentVersionRef {
    DocumentVersionRef {
        id: record.id,
        version: record.version,
    }
}

pub fn values(record: &DocumentRecord) -> PrecautionaryHearingValues {
    PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Imposition,
        scheduled_at: HearingTime::new(OffsetDateTime::UNIX_EPOCH).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court A").unwrap(),
        note: None,
        participants: Vec::new(),
        scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
            HearingNote::new("Declared appointment").unwrap(),
            HearingSupportRef::new(reference(record), record.digest),
            HearingNote::new("Page 1").unwrap(),
        ),
        review_targets: Vec::new(),
    })
    .unwrap()
}

#[derive(Clone, Copy)]
pub enum Failure {
    FormatRejected,
    ValidationLimit,
    Port,
}

pub struct Validator {
    expected: DocumentVersionRef,
    observations: Arc<Mutex<Observations>>,
    calls: AtomicUsize,
    pub formats: Vec<StageDocumentFormat>,
    pub failure: Option<Failure>,
}

impl Validator {
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl DocumentFormatBatchValidator for Validator {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut observed = self.observations.lock().unwrap();
        assert_eq!(batch.inputs().len(), 1);
        assert_eq!(batch.total_bytes(), CONTENT.len());
        assert_eq!(batch.inputs()[0].reference(), self.expected);
        assert_eq!(batch.inputs()[0].bytes(), CONTENT);
        assert_eq!(observed.pointers.len(), 1);
        assert_eq!(
            batch.inputs()[0].bytes().as_ptr() as usize,
            observed.pointers[0]
        );
        observed.events.push("parser");
        match self.failure {
            Some(Failure::FormatRejected) => Err(ApplicationError::StageSupportFormatRejected),
            Some(Failure::ValidationLimit) => Err(ApplicationError::StageSupportValidationLimit),
            Some(Failure::Port) => Err(ApplicationError::Port("format adapter unavailable".into())),
            None => Ok(self.formats.clone()),
        }
    }
}

pub struct Fixture {
    processor: DocumentProcessor,
    observations: Arc<Mutex<Observations>>,
    pub validator: Validator,
}

impl Fixture {
    pub fn processor(&self) -> &DocumentProcessor {
        &self.processor
    }

    pub fn new(record: &DocumentRecord) -> Self {
        let (processor, observations) = observed_crypto::processor();
        Self {
            processor,
            validator: Validator {
                expected: reference(record),
                observations: observations.clone(),
                calls: AtomicUsize::new(0),
                formats: vec![StageDocumentFormat::Pdf],
                failure: None,
            },
            observations,
        }
    }

    pub fn admit(
        &self,
        values: &PrecautionaryHearingValues,
        records: &[DocumentRecord],
    ) -> Result<StageSupportSnapshot, ApplicationError> {
        admit_precautionary_support(
            values,
            records,
            self.processor(),
            &StageSupportReadLimits::standard(),
            &self.validator,
        )
    }

    pub fn events(&self) -> Vec<&'static str> {
        self.observations.lock().unwrap().events.clone()
    }

    pub fn aad(&self) -> Vec<Vec<u8>> {
        self.observations.lock().unwrap().aad.clone()
    }

    pub fn assert_no_crypto_or_parser(&self) {
        assert!(self.events().is_empty());
        assert_eq!(self.validator.calls(), 0);
    }
}
