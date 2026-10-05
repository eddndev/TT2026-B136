use super::*;
use mockall::mock;

mock! {
    pub Reads {}
    impl PrecautionaryHearingRecordReadStore for Reads {
        fn list(&self, actor: &Principal, case_id: CaseId, query: PrecautionaryHearingReadQuery) -> Result<PrecautionaryHearingRecordPage, ApplicationError>;
        fn get(&self, actor: &Principal, case_id: CaseId, hearing: PrecautionaryHearingId, revision: Option<PrecautionaryHearingRevision>) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError>;
        fn get_operation(&self, actor: &Principal, case_id: CaseId, operation: PrecautionaryHearingOperationId) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError>;
    }
}

#[derive(Clone, Copy)]
pub enum ReadKind {
    List,
    Current,
    Exact,
    Operation,
}
pub const READS: [ReadKind; 4] = [
    ReadKind::List,
    ReadKind::Current,
    ReadKind::Exact,
    ReadKind::Operation,
];

pub fn successful_store(
    actor: &Principal,
    expected: &PrecautionaryHearingRecordStoredOperation,
    returned: PrecautionaryHearingRecordStoredOperation,
    kind: ReadKind,
) -> MockReads {
    let mut store = MockReads::new();
    let actor = actor.clone();
    let review = &expected.capture.review;
    let case = review.case_id;
    match kind {
        ReadKind::List => {
            store.expect_list().times(1).return_once(move |a, c, q| {
                assert_eq!(
                    (a, c, q),
                    (&actor, case, PrecautionaryHearingReadQuery::default())
                );
                Ok(page(case, vec![returned]))
            });
        }
        ReadKind::Current | ReadKind::Exact => {
            let hearing = review.command.hearing_id;
            let revision = if matches!(kind, ReadKind::Exact) {
                Some(review.result_revision)
            } else {
                None
            };
            store.expect_get().times(1).return_once(move |a, c, h, r| {
                assert_eq!((a, c, h, r), (&actor, case, hearing, revision));
                Ok(returned)
            });
        }
        ReadKind::Operation => {
            let operation = review.command.operation_id;
            store
                .expect_get_operation()
                .times(1)
                .return_once(move |a, c, o| {
                    assert_eq!((a, c, o), (&actor, case, operation));
                    Ok(returned)
                });
        }
    }
    store
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
pub fn service(
    store: MockReads,
    identity: crate::case_support::MockIdentity,
    clock: Arc<dyn Clock + Send + Sync>,
) -> PrecautionaryHearingRecordReadService {
    PrecautionaryHearingRecordReadService::new(
        Arc::new(store),
        Arc::new(identity),
        Arc::new(Hasher),
        clock,
    )
}
pub struct ReadClock(pub Mutex<Vec<OffsetDateTime>>);
impl Clock for ReadClock {
    fn now(&self) -> OffsetDateTime {
        let mut times = self.0.lock().unwrap();
        if times.len() == 1 {
            times[0]
        } else {
            times.remove(0)
        }
    }
}
pub fn clock() -> Arc<dyn Clock + Send + Sync> {
    Arc::new(ReadClock(Mutex::new(vec![now()])))
}

pub fn read(
    service: &dyn PrecautionaryHearingRecordReadWorkflow,
    expected: &PrecautionaryHearingRecordStoredOperation,
    kind: ReadKind,
) -> Result<Vec<PrecautionaryHearingRecordStoredOperation>, ApplicationError> {
    let r = &expected.capture.review;
    match kind {
        ReadKind::List => service
            .list(
                "session",
                r.case_id,
                PrecautionaryHearingReadQuery::default(),
            )
            .map(|p| p.items),
        ReadKind::Current => service
            .get("session", r.case_id, r.command.hearing_id, None)
            .map(|p| vec![p]),
        ReadKind::Exact => service
            .get(
                "session",
                r.case_id,
                r.command.hearing_id,
                Some(r.result_revision),
            )
            .map(|p| vec![p]),
        ReadKind::Operation => service
            .get_operation("session", r.case_id, r.command.operation_id)
            .map(|p| vec![p]),
    }
}
