use crate::case_support::{CountingClock, MockIdentity};
use application::members::*;
use application::ApplicationError;
use mockall::mock;
use std::sync::Arc;

mock! {
    pub Store {}
    impl MemberStore for Store {
        fn list(&self, actor: UserId, query: UserQuery, at: OffsetDateTime) -> Result<UserPage, ApplicationError>;
        fn get(&self, actor: UserId, id: UserId, at: OffsetDateTime) -> Result<UserSummary, ApplicationError>;
        fn change_access(&self, actor: UserId, id: UserId, change: UserAccessChange, at: OffsetDateTime) -> Result<UserSummary, ApplicationError>;
        fn list_case_members(&self, actor: UserId, id: CaseId, query: CaseMemberQuery, at: OffsetDateTime) -> Result<CaseMemberPage, ApplicationError>;
    }
}

pub fn service(store: MockStore, identity: MockIdentity) -> MemberService {
    MemberService::new(
        Arc::new(store),
        Arc::new(identity),
        Arc::new(CountingClock::default()),
    )
}

pub fn user(id: UserId) -> UserSummary {
    UserSummary {
        id,
        email: "staff@example.com".into(),
        role: Role::Paralegal,
        active: true,
        revision: 7,
    }
}

pub fn query() -> UserQuery {
    UserQuery::new(10, UserStatusFilter::All, None, None, None).unwrap()
}

pub fn page(items: Vec<UserSummary>) -> UserPage {
    UserPage {
        items,
        has_more: false,
        next_cursor: None,
    }
}
