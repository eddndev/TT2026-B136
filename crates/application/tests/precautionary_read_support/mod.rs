pub use crate::context_support::Hasher;
pub use application::{identity::Principal, precautionary_hearings::*, ApplicationError};
pub use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::Sha256Digest,
    identity::{Role, UserId},
    precautionary_hearings::*,
};
use mockall::mock;
use std::sync::{Arc, Mutex};

mock! {
    pub Reads {}
    impl PrecautionaryHearingReadStore for Reads {
        fn list(&self, actor: &Principal, case_id: CaseId, query: PrecautionaryHearingReadQuery) -> Result<PrecautionaryHearingPage, ApplicationError>;
        fn get(&self, actor: &Principal, case_id: CaseId, hearing: PrecautionaryHearingId, revision: Option<PrecautionaryHearingRevision>) -> Result<PrecautionaryHearingStoredOperation, ApplicationError>;
        fn get_operation(&self, actor: &Principal, case_id: CaseId, operation: PrecautionaryHearingOperationId) -> Result<PrecautionaryHearingStoredOperation, ApplicationError>;
    }
}

pub fn id(value: u128) -> PrecautionaryHearingId {
    PrecautionaryHearingId::from_uuid(uuid::Uuid::from_u128(value))
}

pub fn at() -> OffsetDateTime {
    crate::receipt_support::at()
}

pub fn operation(value: u128) -> PrecautionaryHearingStoredOperation {
    let mut fixture = crate::receipt_support::Fixture::schedule();
    fixture.command.hearing_id = id(value);
    fixture.command.operation_id =
        PrecautionaryHearingOperationId::from_uuid(uuid::Uuid::from_u128(1000 + value));
    stored(fixture.capture(None, at()))
}

pub fn stored(capture: PrecautionaryHearingCapture) -> PrecautionaryHearingStoredOperation {
    PrecautionaryHearingStoredOperation {
        history: PrecautionaryHearingHistoryEvidence {
            origin: precautionary_hearing_origin(&Hasher, &capture).unwrap(),
            captures: vec![capture.clone()],
            measure_history: application::precautionary_measures::MeasureHistoryEvidence {
                groups: vec![],
            },
        },
        capture,
    }
}

pub fn replaced(
    previous: &PrecautionaryHearingStoredOperation,
) -> PrecautionaryHearingStoredOperation {
    let mut fixture = crate::receipt_support::Fixture::replace(&previous.capture);
    fixture.command.hearing_id = previous.capture.review.command.hearing_id;
    let capture = fixture.capture(Some(&previous.capture), at() + time::Duration::seconds(2));
    let mut history = previous.history.clone();
    history.captures.push(capture.clone());
    PrecautionaryHearingStoredOperation { capture, history }
}

pub fn refresh(capture: &mut PrecautionaryHearingCapture) {
    use domain::crypto::DocumentHasher;
    capture.review.submission_digest = Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &capture.review.actor,
            capture.review.case_id,
            &capture.review.command,
            &capture.review.resolved_values,
        )
        .unwrap(),
    );
    capture.review.review_digest =
        Hasher.hash_bytes(&precautionary_hearing_review_bytes(&capture.review).unwrap());
    capture.capture_digest =
        Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture).unwrap());
}

pub fn page(
    case_id: CaseId,
    items: Vec<PrecautionaryHearingStoredOperation>,
) -> PrecautionaryHearingPage {
    PrecautionaryHearingPage {
        case_id,
        items,
        has_more: false,
        next_after_id: None,
    }
}

#[derive(Clone, Copy)]
pub enum ReadKind {
    List,
    Get,
    Operation,
}
pub const READS: [ReadKind; 3] = [ReadKind::List, ReadKind::Get, ReadKind::Operation];

pub fn successful_store(
    actor: &Principal,
    expected: &PrecautionaryHearingStoredOperation,
    returned: PrecautionaryHearingStoredOperation,
    kind: ReadKind,
) -> MockReads {
    let mut store = MockReads::new();
    let actor = actor.clone();
    let review = &expected.capture.review;
    let case_id = review.case_id;
    match kind {
        ReadKind::List => {
            store
                .expect_list()
                .times(1)
                .return_once(move |a, c, query| {
                    assert_eq!((a, c), (&actor, case_id));
                    assert_eq!(query, PrecautionaryHearingReadQuery::default());
                    Ok(page(case_id, vec![returned]))
                });
        }
        ReadKind::Get => {
            let (hearing, revision) = (review.command.hearing_id, review.result_revision);
            store.expect_get().times(1).return_once(move |a, c, h, r| {
                assert_eq!((a, c, h, r), (&actor, case_id, hearing, Some(revision)));
                Ok(returned)
            });
        }
        ReadKind::Operation => {
            let operation = review.command.operation_id;
            store
                .expect_get_operation()
                .times(1)
                .return_once(move |a, c, o| {
                    assert_eq!((a, c, o), (&actor, case_id, operation));
                    Ok(returned)
                });
        }
    }
    store
}

pub fn identity(actor: &Principal, calls: usize) -> crate::case_support::MockIdentity {
    let mut identity = crate::case_support::MockIdentity::new();
    let actor = actor.clone();
    identity
        .expect_authenticate()
        .times(calls)
        .withf(|token| token == "session")
        .returning(move |_| Ok(actor.clone()));
    identity
}

pub fn service(
    store: MockReads,
    identity: crate::case_support::MockIdentity,
    clock: Arc<dyn Clock + Send + Sync>,
) -> PrecautionaryHearingReadService {
    PrecautionaryHearingReadService::new(
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
    Arc::new(ReadClock(Mutex::new(vec![
        at() + time::Duration::seconds(100),
    ])))
}

pub fn read(
    service: &dyn PrecautionaryHearingReadWorkflow,
    request: &PrecautionaryHearingStoredOperation,
    kind: ReadKind,
) -> Result<Vec<PrecautionaryHearingStoredOperation>, ApplicationError> {
    let review = &request.capture.review;
    match kind {
        ReadKind::List => service
            .list(
                "session",
                review.case_id,
                PrecautionaryHearingReadQuery::default(),
            )
            .map(|page| page.items),
        ReadKind::Get => service
            .get(
                "session",
                review.case_id,
                review.command.hearing_id,
                Some(review.result_revision),
            )
            .map(|operation| vec![operation]),
        ReadKind::Operation => service
            .get_operation("session", review.case_id, review.command.operation_id)
            .map(|operation| vec![operation]),
    }
}
