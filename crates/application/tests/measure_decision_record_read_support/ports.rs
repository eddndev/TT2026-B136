use super::*;
use application::identity::IdentityWorkflow;
use mockall::mock;
use std::sync::Arc;

mock! {
    pub Reads {}
    impl MeasureDecisionRecordReadStore for Reads {
        fn list(&self, actor: &Principal, case_id: CaseId, query: MeasureDecisionReadQuery) -> Result<MeasureDecisionRecordPage, ApplicationError>;
        fn get(&self, actor: &Principal, case_id: CaseId, decision: MeasureDecisionId) -> Result<MeasureDecisionRecordReceipt, ApplicationError>;
        fn get_operation(&self, actor: &Principal, case_id: CaseId, operation: MeasureDecisionOperationId) -> Result<MeasureDecisionRecordReceipt, ApplicationError>;
    }
}

pub fn identity(actor: &Principal) -> crate::case_support::MockIdentity {
    let mut identity = crate::case_support::MockIdentity::new();
    let actor = actor.clone();
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

pub fn service(
    store: MockReads,
    identity: impl IdentityWorkflow + 'static,
) -> MeasureDecisionRecordReadService {
    service_with_clock(store, identity, Arc::new(FixedClock(now())))
}

pub fn service_with_clock(
    store: MockReads,
    identity: impl IdentityWorkflow + 'static,
    clock: Arc<dyn Clock + Send + Sync>,
) -> MeasureDecisionRecordReadService {
    MeasureDecisionRecordReadService::new(
        Arc::new(store),
        Arc::new(identity),
        Arc::new(Hasher),
        clock,
    )
}
