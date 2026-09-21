//! PostgreSQL directory and account access persistence.
mod mutation;
mod query;
mod values;
use application::members::{
    CaseMemberPage, CaseMemberQuery, MemberStore, UserAccessChange, UserPage, UserQuery,
    UserSummary,
};
use application::ApplicationError;
use domain::{cases::CaseId, clock::Clock, identity::UserId};
use postgres::Client;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;

pub struct PostgresMemberStore {
    client: Mutex<Client>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PostgresMemberStore {
    pub fn open(
        database_url: &str,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(database_url)?),
            clock,
        })
    }
    fn client(&self) -> Result<std::sync::MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| ApplicationError::Port("member database lock poisoned".into()))
    }
    fn checked_at(&self, lower: OffsetDateTime) -> Result<OffsetDateTime, ApplicationError> {
        let at = self.clock.now();
        if lower.offset() != time::UtcOffset::UTC
            || at.offset() != time::UtcOffset::UTC
            || at < lower
            || !(1..=9999).contains(&at.year())
        {
            return Err(application::members::MemberError::Stored(
                "invalid member observation clock",
            )
            .into());
        }
        Ok(at)
    }
}

impl MemberStore for PostgresMemberStore {
    fn list(
        &self,
        actor: UserId,
        query: UserQuery,
        at: OffsetDateTime,
    ) -> Result<UserPage, ApplicationError> {
        self.list_users(actor, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        id: UserId,
        at: OffsetDateTime,
    ) -> Result<UserSummary, ApplicationError> {
        self.read_user(actor, id, at)
    }
    fn change_access(
        &self,
        actor: UserId,
        id: UserId,
        change: UserAccessChange,
        at: OffsetDateTime,
    ) -> Result<UserSummary, ApplicationError> {
        self.change(actor, id, change, at)
    }
    fn list_case_members(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: CaseMemberQuery,
        at: OffsetDateTime,
    ) -> Result<CaseMemberPage, ApplicationError> {
        self.case_members(actor, case_id, query, at)
    }
}

fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("member database: {error}"))
}
