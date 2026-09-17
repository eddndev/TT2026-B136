use application::case_stages::{CaseStageEntry, CurrentCaseStage};
use application::cases::*;
use application::documents::{
    DocumentFormatBatch, DocumentFormatBatchValidator, DocumentRecord, StageDocumentFormat,
    StageSupportReadLimits,
};
use application::hearings::*;
use application::ApplicationError;
use domain::{
    cases::{CaseId, CaseMetadata},
    clock::OffsetDateTime,
    crypto::DocumentHasher,
    identity::UserId,
};
use mockall::mock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[allow(unused_imports)]
pub use crate::case_support::{identity, instant, CountingClock, MockIdentity};
mod scenarios;
#[allow(unused_imports)]
pub use scenarios::*;

mock! {
    pub Store {}
    impl HearingStore for Store {
        fn context(&self, actor:UserId, case_id:CaseId, at:OffsetDateTime)->Result<HearingCaseContext,ApplicationError>;
        fn list(&self, actor:UserId, case_id:CaseId, query:HearingQuery, at:OffsetDateTime)->Result<HearingPage,ApplicationError>;
        fn get(&self, actor:UserId, case_id:CaseId, id:HearingId, revision:Option<HearingRevision>, at:OffsetDateTime)->Result<HearingDetail,ApplicationError>;
        fn history(&self, actor:UserId, case_id:CaseId, id:HearingId, query:HearingHistoryQuery, at:OffsetDateTime)->Result<HearingHistoryPage,ApplicationError>;
        fn agenda(&self, actor:UserId, query:HearingAgendaQuery, at:OffsetDateTime)->Result<HearingAgendaPage,ApplicationError>;
        fn prepare(&self, actor:UserId, case_id:CaseId, command:&HearingCommand, limits:&StageSupportReadLimits)->Result<HearingPreparation,ApplicationError>;
        fn commit(&self, actor:UserId, case_id:CaseId, prepared:PreparedHearingChange)->Result<HearingDetail,ApplicationError>;
    }
}

pub fn hasher() -> Arc<dyn DocumentHasher + Send + Sync> {
    Arc::from(crate::crypto::processor_ports().hasher)
}

#[derive(Default)]
pub struct Validator {
    pub calls: AtomicUsize,
    pub failure: bool,
}
impl DocumentFormatBatchValidator for Validator {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.failure {
            return Err(ApplicationError::StageSupportFormatRejected);
        }
        assert!(batch.inputs().iter().all(|input| !input.bytes().is_empty()));
        Ok(vec![StageDocumentFormat::Pdf; batch.inputs().len()])
    }
}

pub fn service(
    store: MockStore,
    identity: MockIdentity,
    validator: Arc<Validator>,
) -> (HearingService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        HearingService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(crate::crypto::processor()),
            validator,
            hasher(),
            clock.clone(),
        ),
        clock,
    )
}

pub fn expectation() -> HearingContextExpectation {
    HearingContextExpectation {
        case_revision: CaseRevision::FIRST,
        stage_revision: CaseStageRevision::FIRST,
    }
}

pub fn values() -> HearingValues {
    HearingValues::new(HearingValuesInput {
        kind: HearingKind::Initial,
        scheduled_at: HearingTime::new(instant()).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court A").unwrap(),
        note: None,
        participants: vec![],
        conviction_basis: None,
    })
    .unwrap()
}

pub fn command() -> HearingCommand {
    HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: HearingId::new(),
        change: HearingChange::Schedule {
            context: expectation(),
            values: values(),
        },
    }
}

pub fn context(case_id: CaseId, actor: UserId) -> HearingCaseContext {
    let values = PenalCaseCreation::new(
        CaseMetadata::new("Case", "REF").unwrap(),
        PenalCaseProfile::new("NUC", "Authority", "CJ", "Court", &["Offense"], None, None).unwrap(),
    )
    .into_values();
    let digest = case_administration_digest(hasher().as_ref(), &values);
    let actor = CaseActorSnapshot {
        id: actor,
        email: "actor@example.com".into(),
    };
    HearingCaseContext {
        case_id,
        administration: CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
            case_id,
            revision: CaseRevision::FIRST,
            values,
            values_digest: digest,
            changed_at: instant(),
            changed_by: actor.clone(),
        })),
        stage: CurrentCaseStage::Registered(Box::new(CaseStageEntry::Initial(
            CaseInitialStageRegistration {
                case_id,
                stage_revision: CaseStageRevision::FIRST,
                administration_revision: CaseRevision::FIRST,
                stage: InitialCaseStage::Investigation,
                administration_digest: digest,
                recorded_at: instant(),
                recorded_by: actor,
            },
        ))),
    }
}

pub fn preparation(case_id: CaseId, actor: UserId) -> HearingPreparation {
    HearingPreparation {
        base: None,
        context: context(case_id, actor),
        participants: vec![],
        records: vec![],
    }
}

pub fn scheduling(context: &HearingCaseContext) -> HearingSchedulingContext {
    let admin = context.administration.snapshot().unwrap();
    let stage = context.stage.entry().unwrap();
    HearingSchedulingContext {
        administration_revision: admin.revision,
        administration_digest: admin.values_digest,
        stage_revision: stage.stage_revision(),
        stage: stage.stage(),
        stage_digest: match stage {
            CaseStageEntry::Initial(_) => None,
            CaseStageEntry::Changed(value) => Some(value.values_digest),
        },
    }
}

pub fn detail(
    case_id: CaseId,
    actor: UserId,
    command: &HearingCommand,
    values: HearingValues,
    context: &HearingCaseContext,
) -> HearingDetail {
    let digest = hearing_values_digest(hasher().as_ref(), &values);
    HearingDetail {
        snapshot: HearingSnapshot {
            case_id,
            id: command.hearing_id,
            revision: command.result_revision().unwrap(),
            values,
            values_digest: digest,
            status: if command.action() == HearingAction::Cancel {
                HearingStatus::Cancelled
            } else {
                HearingStatus::Scheduled
            },
            reason: command.reason().cloned(),
            receipt: HearingReceipt {
                operation_id: command.operation_id,
                action: command.action(),
                expected_revision: command.expected_revision(),
                expected_context: command.expected_context(),
                submission_digest: hearing_submission_digest(
                    hasher().as_ref(),
                    actor,
                    case_id,
                    command,
                    digest,
                ),
            },
            scheduling_context: scheduling(context),
            recorded_administration_revision: context.administration.revision().unwrap(),
            recorded_administration_digest: context
                .administration
                .snapshot()
                .unwrap()
                .values_digest,
            recorded_at: instant(),
            recorded_by: CaseActorSnapshot {
                id: actor,
                email: "actor@example.com".into(),
            },
        },
        participants: vec![],
        support: None,
    }
}

pub fn replacement(base: &HearingDetail) -> HearingCommand {
    HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: base.snapshot.id,
        change: HearingChange::Replace {
            expected_revision: base.snapshot.revision,
            context: expectation(),
            values: base.snapshot.values.clone(),
            reason: HearingNote::new("Rescheduled").unwrap(),
        },
    }
}

pub fn cancellation(base: &HearingDetail) -> HearingCommand {
    HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: base.snapshot.id,
        change: HearingChange::Cancel {
            expected_revision: base.snapshot.revision,
            reason: HearingNote::new("Cancelled by office").unwrap(),
        },
    }
}

pub fn values_input(values: &HearingValues) -> HearingValuesInput {
    HearingValuesInput {
        kind: values.kind(),
        scheduled_at: values.scheduled_at(),
        modality: values.modality(),
        venue: values.venue().clone(),
        note: values.note().cloned(),
        participants: values.participants().to_vec(),
        conviction_basis: values.conviction_basis().cloned(),
    }
}

pub fn support(record: &DocumentRecord) -> HearingSupportRef {
    HearingSupportRef::new(
        domain::crypto::DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        record.digest,
    )
}
