use super::*;
use application::documents::{
    DocumentFormatBatch, DocumentFormatBatchValidator, StageDocumentFormat, StageSupportReadLimits,
};
use application::identity::{IdentityWorkflow, Principal};
use domain::cases::CaseId;
use domain::clock::{Clock, OffsetDateTime};
use mockall::mock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

mock! {
    pub Store {}
    impl MeasureDecisionStore for Store {
        fn prepare(&self, actor: &Principal, case_id: CaseId, command: &MeasureDecisionCommand, limits: &StageSupportReadLimits) -> Result<MeasureDecisionPreparation, ApplicationError>;
        fn commit(&self, actor: &Principal, case_id: CaseId, prepared: PreparedMeasureDecision) -> Result<MeasureDecisionStoredOperation, ApplicationError>;
    }
}

pub fn identity(actor: Principal) -> crate::case_support::MockIdentity {
    let mut identity = crate::case_support::MockIdentity::new();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .returning(move |_| Ok(actor.clone()));
    identity
}

pub struct FixedClock(pub OffsetDateTime);
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        self.0
    }
}

#[derive(Default)]
pub struct Validator {
    calls: AtomicUsize,
    pub failure: bool,
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
        if self.failure {
            return Err(ApplicationError::StageSupportFormatRejected);
        }
        Ok(vec![StageDocumentFormat::Pdf; batch.inputs().len()])
    }
}

pub struct Harness {
    pub service: MeasureDecisionService,
    pub validator: Arc<Validator>,
    pub observations: Arc<Mutex<crate::observed_crypto::Observations>>,
}
impl Harness {
    pub fn events(&self) -> Vec<&'static str> {
        self.observations.lock().unwrap().events.clone()
    }
}

pub fn harness(store: MockStore, identity: impl IdentityWorkflow + 'static) -> Harness {
    harness_with(
        store,
        identity,
        Validator::default(),
        Arc::new(FixedClock(now())),
    )
}

pub fn harness_with(
    store: MockStore,
    identity: impl IdentityWorkflow + 'static,
    validator: Validator,
    clock: Arc<dyn Clock + Send + Sync>,
) -> Harness {
    let (processor, observations) = crate::observed_crypto::processor();
    let validator = Arc::new(validator);
    Harness {
        service: MeasureDecisionService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(processor),
            validator.clone(),
            Arc::new(Hasher),
            clock,
        ),
        validator,
        observations,
    }
}
