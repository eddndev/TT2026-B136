pub use crate::context_support::{at, Hasher};
pub use application::{
    identity::Principal,
    precautionary_hearings::{
        PrecautionaryContext, PrecautionaryContextReadService, PrecautionaryContextReadStore,
        PrecautionaryContextReadWorkflow,
    },
    ApplicationError,
};
pub use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    identity::{Role, UserId},
};
use mockall::mock;
pub use std::sync::{Arc, Mutex};

mock! {
    pub Reads {}
    impl PrecautionaryContextReadStore for Reads {
        fn get(&self, actor: &Principal, case_id: CaseId) -> Result<PrecautionaryContext, ApplicationError>;
    }
}

pub fn reader(role: Role) -> Principal {
    Principal {
        id: UserId::from_uuid(uuid::Uuid::from_u128(900)),
        email: "current-reader@example.test".into(),
        role,
    }
}
pub fn now() -> OffsetDateTime {
    at() + time::Duration::seconds(100)
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
pub fn identity(actor: &Principal) -> crate::case_support::MockIdentity {
    let mut identity = crate::case_support::MockIdentity::new();
    let actor = actor.clone();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .returning(move |_| Ok(actor.clone()));
    identity
}
pub fn returning(actor: &Principal, case_id: CaseId, context: PrecautionaryContext) -> MockReads {
    let mut store = MockReads::new();
    let actor = actor.clone();
    store
        .expect_get()
        .times(1)
        .return_once(move |actual, case| {
            assert_eq!((actual, case), (&actor, case_id));
            Ok(context)
        });
    store
}
pub fn service(
    store: MockReads,
    identity: crate::case_support::MockIdentity,
    clock: Arc<dyn Clock + Send + Sync>,
) -> PrecautionaryContextReadService {
    PrecautionaryContextReadService::new(
        Arc::new(store),
        Arc::new(identity),
        Arc::new(Hasher),
        clock,
    )
}
pub fn context() -> PrecautionaryContext {
    PrecautionaryContext::new(&Hasher, crate::context_support::initial()).unwrap()
}

mod authorization;
mod validation;
