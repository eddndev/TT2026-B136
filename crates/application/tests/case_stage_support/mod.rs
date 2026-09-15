use application::case_stages::*;
use application::cases::{CaseActorSnapshot, CaseInitialStageRegistration};
use application::documents::{
    DocumentFormatBatch, DocumentFormatBatchValidator, DocumentRecord, DocumentVersionRef,
};
use application::ApplicationError;
use domain::case_administration::{CaseRevision, InitialCaseStage};
use domain::cases::CaseId;
use domain::clock::{Clock, OffsetDateTime};
use domain::crypto::Sha256Digest;
use domain::identity::UserId;
use mockall::mock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub use super::case_support::{identity, MockIdentity};

mock! {
    pub Store {}
    impl CaseStageStore for Store {
        fn get(&self, actor:UserId, case_id:CaseId, at:OffsetDateTime)->Result<CaseStageDetail,ApplicationError>;
        fn history(&self, actor:UserId, case_id:CaseId, query:CaseStageQuery, at:OffsetDateTime)->Result<CaseStagePage,ApplicationError>;
        fn prepare(&self, actor:UserId, case_id:CaseId, expected:CaseStageExpectation, change:&CaseStageChange, limits:&StageSupportReadLimits)->Result<CaseStagePreparation,ApplicationError>;
        fn commit(&self, actor:UserId, case_id:CaseId, expected:CaseStageExpectation, prepared:PreparedCaseStageChange, at:OffsetDateTime)->Result<CaseStageDetail,ApplicationError>;
    }
}

pub struct Validator {
    pub calls: AtomicUsize,
    pub expected: Vec<DocumentVersionRef>,
    pub failure: bool,
}
impl DocumentFormatBatchValidator for Validator {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            batch
                .inputs()
                .iter()
                .map(|input| input.reference())
                .collect::<Vec<_>>(),
            self.expected
        );
        if self.failure {
            return Err(ApplicationError::StageSupportFormatRejected);
        }
        Ok(batch
            .inputs()
            .iter()
            .enumerate()
            .map(|(index, _)| {
                if index == 0 {
                    StageDocumentFormat::Pdf
                } else {
                    StageDocumentFormat::Docx
                }
            })
            .collect())
    }
}
pub fn validator(records: &[DocumentRecord]) -> Arc<Validator> {
    Arc::new(Validator {
        calls: AtomicUsize::new(0),
        expected: records.iter().map(|r| reference(r).reference()).collect(),
        failure: false,
    })
}
pub fn instant() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(1_800_000_000_123_456_789).unwrap()
}
#[derive(Default)]
pub struct CountingClock(pub AtomicUsize);
impl Clock for CountingClock {
    fn now(&self) -> OffsetDateTime {
        self.0.fetch_add(1, Ordering::SeqCst);
        instant()
    }
}
impl CountingClock {
    pub fn calls(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}
pub fn service(
    store: MockStore,
    identity: MockIdentity,
    validator: Arc<Validator>,
) -> (CaseStageService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        CaseStageService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(super::crypto::processor()),
            validator,
            clock.clone(),
        ),
        clock,
    )
}
pub fn reference(record: &DocumentRecord) -> StageSupportRef {
    StageSupportRef::new(
        DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        record.digest,
    )
}
pub fn adoption(record: &DocumentRecord, stage: CaseStage) -> StageAdoption {
    StageAdoption::new(
        stage,
        DeclaredStageTime::instant(instant()).unwrap(),
        StageNote::new("Known history").unwrap(),
        reference(record),
    )
}
pub fn initial(case_id: CaseId, actor: UserId) -> CurrentCaseStage {
    CurrentCaseStage::Registered(Box::new(CaseStageEntry::Initial(
        CaseInitialStageRegistration {
            case_id,
            stage_revision: CaseStageRevision::FIRST,
            administration_revision: CaseRevision::FIRST,
            stage: InitialCaseStage::Investigation,
            administration_digest: Sha256Digest::from_array([3; 32]),
            recorded_at: instant(),
            recorded_by: CaseActorSnapshot {
                id: actor,
                email: "original@example.com".into(),
            },
        },
    )))
}
pub fn committed(
    case_id: CaseId,
    actor: UserId,
    revision: u32,
    change: CaseStageChange,
) -> CaseStageDetail {
    let from_stage = match &change {
        CaseStageChange::Adopt(_) => None,
        CaseStageChange::Transition(StageTransition::ToIntermediate(_)) => {
            Some(CaseStage::Investigation)
        }
        CaseStageChange::Transition(StageTransition::ToTrial(_)) => Some(CaseStage::Intermediate),
    };
    CaseStageDetail {
        case_id,
        current: CurrentCaseStage::Registered(Box::new(CaseStageEntry::Changed(Box::new(
            CaseStageSnapshot {
                case_id,
                stage_revision: CaseStageRevision::new(revision).unwrap(),
                from_stage,
                values: change,
                values_digest: Sha256Digest::from_array([7; 32]),
                administration_revision: CaseRevision::new(7).unwrap(),
                administration_digest: Sha256Digest::from_array([8; 32]),
                supports: Vec::new(),
                recorded_at: instant(),
                recorded_by: CaseActorSnapshot {
                    id: actor,
                    email: "commit@example.com".into(),
                },
            },
        )))),
    }
}
