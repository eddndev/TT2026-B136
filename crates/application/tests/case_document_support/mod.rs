use std::sync::Arc;

use application::documents::{
    CaseDocumentService, CaseDocumentStore, DocumentAction, DocumentRecord,
};
use application::identity::{
    EnrollmentResult, IdentityWorkflow, LoginChallenge, Principal, SessionResult,
};
use application::ApplicationError;
use domain::audit::ChainedEvent;
use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::crypto::DocumentId;
use domain::identity::{Permission, Role, UserId};
use mockall::mock;

mock! {
    pub Store {}
    impl CaseDocumentStore for Store {
        fn check_access(&self, actor: UserId, case: CaseId, action: DocumentAction) -> Result<(), ApplicationError>;
        fn load(&self, actor: UserId, case: CaseId, id: DocumentId, action: DocumentAction) -> Result<DocumentRecord, ApplicationError>;
        fn insert(&self, actor: UserId, case: CaseId, record: DocumentRecord, at: OffsetDateTime) -> Result<(), ApplicationError>;
        fn seal(&self, actor: UserId, case: CaseId, record: DocumentRecord, at: OffsetDateTime) -> Result<(), ApplicationError>;
        fn record_access(&self, actor: UserId, case: CaseId, record: &DocumentRecord, action: DocumentAction, at: OffsetDateTime) -> Result<(), ApplicationError>;
        fn audit_entries(&self, actor: UserId) -> Result<Vec<ChainedEvent>, ApplicationError>;
    }
}

mock! {
    pub Identity {}
    impl IdentityWorkflow for Identity {
        fn bootstrap_owner(&self, email: &str, password: &str) -> Result<EnrollmentResult, ApplicationError>;
        fn create_user(&self, token: &str, email: &str, password: &str, role: Role) -> Result<EnrollmentResult, ApplicationError>;
        fn start_login(&self, email: &str, password: &str) -> Result<LoginChallenge, ApplicationError>;
        fn complete_totp(&self, token: &str, code: &str) -> Result<SessionResult, ApplicationError>;
        fn complete_recovery(&self, token: &str, code: &str) -> Result<SessionResult, ApplicationError>;
        fn authenticate(&self, token: &str) -> Result<Principal, ApplicationError>;
        fn authorize(&self, token: &str, permission: Permission) -> Result<Principal, ApplicationError>;
        fn logout(&self, token: &str) -> Result<(), ApplicationError>;
    }
}

pub fn identity(role: Role, calls: usize) -> (MockIdentity, UserId) {
    let actor = Principal {
        id: UserId::new(),
        email: "actor@example.com".into(),
        role,
    };
    let id = actor.id;
    let mut identity = MockIdentity::new();
    identity
        .expect_authenticate()
        .times(calls)
        .withf(|token| token == "session")
        .returning(move |_| Ok(actor.clone()));
    (identity, id)
}

pub fn service(store: MockStore, identity: MockIdentity) -> CaseDocumentService {
    CaseDocumentService::new(
        Arc::new(store),
        Arc::new(identity),
        super::crypto::processor(),
        Arc::new(super::crypto::TestClock),
    )
}
