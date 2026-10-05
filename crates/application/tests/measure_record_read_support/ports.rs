use super::*;
use application::identity::IdentityWorkflow;
use mockall::mock;
use std::sync::Arc;

mock! {
    pub Reads {}
    impl MeasureRecordReadStore for Reads {
        fn list(&self, actor: &Principal, case_id: CaseId, query: MeasureRecordReadQuery) -> Result<MeasureRecordPage, ApplicationError>;
        fn get(&self, actor: &Principal, case_id: CaseId, id: MeasureId) -> Result<MeasureRecordDetail, ApplicationError>;
        fn exact(&self, actor: &Principal, case_id: CaseId, reference: PrecautionaryMeasureRef) -> Result<MeasureRecordDetail, ApplicationError>;
    }
}
pub fn identity(actor: &Principal) -> crate::case_support::MockIdentity {
    let mut result = crate::case_support::MockIdentity::new();
    let actor = actor.clone();
    result
        .expect_authenticate()
        .withf(|token| token == "session")
        .returning(move |_| Ok(actor.clone()));
    result
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
) -> MeasureRecordReadService {
    service_with_clock(store, identity, Arc::new(FixedClock(now())))
}
pub fn service_with_clock(
    store: MockReads,
    identity: impl IdentityWorkflow + 'static,
    clock: Arc<dyn Clock + Send + Sync>,
) -> MeasureRecordReadService {
    MeasureRecordReadService::new(Arc::new(store), Arc::new(identity), Arc::new(Hasher), clock)
}
