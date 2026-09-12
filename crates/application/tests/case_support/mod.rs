use application::cases::{CaseAccess, CaseRecord, CaseRepository};
use application::identity::{
    EnrollmentResult, IdentityWorkflow, LoginChallenge, Principal, SessionResult,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::identity::{Permission, Role, UserId};
use mockall::mock;

mock! {
    pub Identity {}
    impl IdentityWorkflow for Identity {
        fn bootstrap_owner(&self, email: &str, password: &str) -> Result<EnrollmentResult, ApplicationError>;
        fn create_user(&self, actor: &Principal, email: &str, password: &str, role: Role) -> Result<EnrollmentResult, ApplicationError>;
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
        fn insert(&self, record: CaseRecord) -> Result<(), ApplicationError>;
        fn list(&self, access: CaseAccess, limit: u32, offset: u32) -> Result<Vec<CaseRecord>, ApplicationError>;
        fn find(&self, id: CaseId, access: CaseAccess) -> Result<Option<CaseRecord>, ApplicationError>;
        fn add_member(&self, id: CaseId, user_id: UserId) -> Result<(), ApplicationError>;
        fn remove_member(&self, id: CaseId, user_id: UserId) -> Result<(), ApplicationError>;
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
