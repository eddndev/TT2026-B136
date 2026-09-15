use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use application::participants::{
    DirectoryStatus, ParticipantActorSnapshot, ParticipantHistoryPage, ParticipantHistoryQuery,
    ParticipantId, ParticipantPage, ParticipantQuery, ParticipantRevision, ParticipantService,
    ParticipantSnapshot, ParticipantStore, ParticipantValues,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::clock::{Clock, OffsetDateTime};
use domain::crypto::Sha256Digest;
use domain::identity::UserId;
use mockall::mock;

pub use super::case_support::{identity, MockIdentity};

mock! {
    pub Store {}
    impl ParticipantStore for Store {
        fn create(&self, actor: UserId, case_id: CaseId, id: ParticipantId, values: ParticipantValues, at: OffsetDateTime) -> Result<ParticipantSnapshot, ApplicationError>;
        fn replace(&self, actor: UserId, case_id: CaseId, id: ParticipantId, expected_revision: ParticipantRevision, values: ParticipantValues, at: OffsetDateTime) -> Result<ParticipantSnapshot, ApplicationError>;
        fn change_status(&self, actor: UserId, case_id: CaseId, id: ParticipantId, expected_revision: ParticipantRevision, status: DirectoryStatus, at: OffsetDateTime) -> Result<ParticipantSnapshot, ApplicationError>;
        fn list(&self, actor: UserId, case_id: CaseId, query: ParticipantQuery, at: OffsetDateTime) -> Result<ParticipantPage, ApplicationError>;
        fn get(&self, actor: UserId, case_id: CaseId, id: ParticipantId, at: OffsetDateTime) -> Result<ParticipantSnapshot, ApplicationError>;
        fn history(&self, actor: UserId, case_id: CaseId, id: ParticipantId, query: ParticipantHistoryQuery, at: OffsetDateTime) -> Result<ParticipantHistoryPage, ApplicationError>;
    }
}

pub fn instant() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(1_735_689_600_123_456_789).unwrap()
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
) -> (ParticipantService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        ParticipantService::new(Arc::new(store), Arc::new(identity), clock.clone()),
        clock,
    )
}

pub fn values() -> ParticipantValues {
    ParticipantValues::new(
        "Ana",
        "Defensa",
        Some("Despacho"),
        None,
        DirectoryStatus::Active,
    )
    .unwrap()
}

pub fn snapshot(
    case_id: CaseId,
    id: ParticipantId,
    actor: UserId,
    revision: u32,
) -> ParticipantSnapshot {
    ParticipantSnapshot {
        case_id,
        id,
        revision: ParticipantRevision::new(revision).unwrap(),
        values: values(),
        values_digest: Sha256Digest::from_array([7; 32]),
        changed_at: instant(),
        changed_by: ParticipantActorSnapshot {
            id: actor,
            email: "captured@example.com".into(),
        },
    }
}
