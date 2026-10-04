use application::{
    identity::Principal, resource_activities::ResourceId, resource_hearings::*, ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    identity::UserId,
    resource_hearings::{ResourceHearingId, ResourceHearingRevision},
};
use mockall::mock;
use std::sync::{Arc, Mutex};

pub use crate::resource_hearing_support::Fixture;

mock! {
    pub Reads {}
    impl ResourceHearingReadStore for Reads {
        fn list(&self, actor: UserId, case: CaseId, resource: ResourceId, query: ResourceHearingReadQuery) -> Result<ResourceHearingPage, ApplicationError>;
        fn get(&self, actor: UserId, case: CaseId, resource: ResourceId, id: ResourceHearingId, revision: Option<ResourceHearingRevision>) -> Result<ResourceHearingCreation, ApplicationError>;
    }
}

pub fn id(value: u128) -> ResourceHearingId {
    ResourceHearingId::from_uuid(uuid::Uuid::from_u128(value))
}

pub fn creation(f: &Fixture, id: ResourceHearingId, at: OffsetDateTime) -> ResourceHearingCreation {
    let mut command = f.command.clone();
    command.hearing_id = id;
    command.operation_id = domain::resource_hearings::ResourceHearingOperationId::new();
    command.association_id = application::resource_activities::ResourceActivityId::new();
    prepare_resource_hearing_change(
        crate::hearing_support::hasher(),
        &f.actor,
        f.case(),
        f.resource(),
        command,
        f.material.clone(),
    )
    .unwrap()
    .into_creation(at)
    .unwrap()
}

pub fn page(f: &Fixture, items: Vec<ResourceHearingCreation>) -> ResourceHearingPage {
    ResourceHearingPage {
        case_id: f.case(),
        resource_id: f.resource(),
        items,
        has_more: false,
        next_after_id: None,
    }
}

pub fn successful_store(
    f: &Fixture,
    actor: &Principal,
    result: ResourceHearingCreation,
    list: bool,
) -> MockReads {
    let mut store = MockReads::new();
    let (actor, case, resource) = (actor.id, f.case(), f.resource());
    if list {
        let result = page(f, vec![result]);
        store.expect_list().times(1).return_once(move |a, c, r, q| {
            assert_eq!((a, c, r), (actor, case, resource));
            assert_eq!(q.limit(), 10);
            assert_eq!(q.after_id(), None);
            Ok(result)
        });
    } else {
        let id = f.command.hearing_id;
        store
            .expect_get()
            .times(1)
            .return_once(move |a, c, r, i, v| {
                assert_eq!((a, c, r, i), (actor, case, resource, id));
                assert_eq!(v, Some(ResourceHearingRevision::initial()));
                Ok(result)
            });
    }
    store
}

pub fn identity(actor: &Principal, calls: usize) -> crate::case_support::MockIdentity {
    crate::procedural_resource_support::identity_for(actor.clone(), calls).0
}

pub fn changed_identity(
    actor: Principal,
    changed: Result<Principal, ApplicationError>,
) -> crate::case_support::MockIdentity {
    let mut identity = crate::case_support::MockIdentity::new();
    let mut order = mockall::Sequence::new();
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| Ok(actor));
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| changed);
    identity
}

pub fn service(
    store: MockReads,
    identity: crate::case_support::MockIdentity,
    clock: Arc<dyn Clock + Send + Sync>,
) -> ResourceHearingReadService {
    ResourceHearingReadService::new(
        Arc::new(store),
        Arc::new(identity),
        crate::hearing_support::hasher(),
        clock,
    )
}

pub fn clock() -> Arc<dyn Clock + Send + Sync> {
    Arc::new(crate::case_support::CountingClock::default())
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

pub fn read(
    service: &dyn ResourceHearingReadWorkflow,
    f: &Fixture,
    list: bool,
) -> Result<Vec<ResourceHearingCreation>, ApplicationError> {
    if list {
        service
            .list(
                "session",
                f.case(),
                f.resource(),
                ResourceHearingReadQuery::default(),
            )
            .map(|page| page.items)
    } else {
        service
            .get(
                "session",
                f.case(),
                f.resource(),
                f.command.hearing_id,
                Some(ResourceHearingRevision::initial()),
            )
            .map(|row| vec![row])
    }
}

pub fn stored_error<T: std::fmt::Debug>(result: Result<T, ApplicationError>) {
    assert!(
        matches!(
            result,
            Err(ApplicationError::ResourceActivity(
                application::resource_activities::ResourceActivityError::StoredInconsistent(_)
            ))
        ),
        "{result:?}"
    );
}
