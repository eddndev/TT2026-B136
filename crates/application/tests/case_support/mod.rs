use application::cases::*;
use application::identity::{
    EnrollmentResult, IdentityWorkflow, LoginChallenge, Principal, SessionResult,
};
use application::ApplicationError;
use domain::cases::{CaseId, CaseMetadata};
use domain::clock::{Clock, OffsetDateTime};
use domain::identity::{Permission, Role, UserId};
use mockall::mock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

mock! {
    pub Identity {}
    impl IdentityWorkflow for Identity {
        fn bootstrap_owner(&self, email: &str, password: &str) -> Result<EnrollmentResult, ApplicationError>;
        fn create_user(&self, access_token: &str, email: &str, password: &str, role: Role) -> Result<EnrollmentResult, ApplicationError>;
        fn start_login(&self, email: &str, password: &str) -> Result<LoginChallenge, ApplicationError>;
        fn complete_totp(&self, token: &str, code: &str) -> Result<SessionResult, ApplicationError>;
        fn complete_recovery(&self, token: &str, code: &str) -> Result<SessionResult, ApplicationError>;
        fn authenticate(&self, token: &str) -> Result<Principal, ApplicationError>;
        fn authorize(&self, token: &str, permission: Permission) -> Result<Principal, ApplicationError>;
        fn logout(&self, token: &str) -> Result<(), ApplicationError>;
    }
}

mock! {
    pub Cases {}
    impl CaseRepository for Cases {
    fn create_basic(&self, actor: UserId, id: CaseId, metadata: CaseMetadata, at: OffsetDateTime) -> Result<CaseRecord, ApplicationError>;
    fn list_basic(&self, actor: UserId, limit: u32, offset: u32, at: OffsetDateTime) -> Result<Vec<CaseRecord>, ApplicationError>;
    fn get_basic(&self, actor: UserId, id: CaseId, at: OffsetDateTime) -> Result<CaseRecord, ApplicationError>;
    fn add_member(&self, id: CaseId, user_id: UserId, actor: UserId, at: OffsetDateTime) -> Result<(), ApplicationError>;
    fn remove_member(&self, id: CaseId, user_id: UserId, actor: UserId, at: OffsetDateTime) -> Result<(), ApplicationError>;
    fn register_penal(&self, actor: UserId, id: CaseId, creation: PenalCaseCreation, at: OffsetDateTime) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn replace_administration(&self, actor: UserId, id: CaseId, expected: CaseRevisionExpectation, values: CaseEditableValues, at: OffsetDateTime) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn change_administrative_status(&self, actor: UserId, id: CaseId, expected: CaseRevisionExpectation, status: CaseAdministrativeStatus, at: OffsetDateTime) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn list_administrations(&self, actor: UserId, query: CaseAdministrationQuery, at: OffsetDateTime) -> Result<CaseAdministrationPage, ApplicationError>;
    fn get_administration(&self, actor: UserId, id: CaseId, at: OffsetDateTime) -> Result<CaseAdministrationDetail, ApplicationError>;
    fn administration_history(&self, actor: UserId, id: CaseId, query: CaseAdministrationHistoryQuery, at: OffsetDateTime) -> Result<CaseAdministrationHistoryPage, ApplicationError>;
    }
}

pub fn identity(role: Role, calls: usize) -> (MockIdentity, Principal) {
    let principal = Principal {
        id: UserId::new(),
        email: "member@example.com".into(),
        role,
    };
    let mut identity = MockIdentity::new();
    let result = principal.clone();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .times(calls)
        .returning(move |_| Ok(result.clone()));
    (identity, principal)
}

pub fn record(created_by: UserId) -> CaseRecord {
    CaseRecord {
        id: CaseId::new(),
        title: "Defense file".into(),
        reference: "NUC-123".into(),
        created_by,
    }
}

pub fn instant() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap()
}

#[derive(Default)]
pub struct CountingClock {
    calls: AtomicUsize,
}
impl CountingClock {
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}
impl Clock for CountingClock {
    fn now(&self) -> OffsetDateTime {
        self.calls.fetch_add(1, Ordering::SeqCst);
        instant()
    }
}
pub fn service(repository: MockCases, identity: MockIdentity) -> (CaseService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        CaseService::new(Arc::new(repository), Arc::new(identity), clock.clone()),
        clock,
    )
}
