//! Exact independent activity associations in one audited transaction.
mod commit;
mod decode;
mod preparation;
mod query;
mod selection;
mod sources;
pub(crate) mod storage;
mod write;
use crate::procedural_resource_postgres::authorize;
use application::{resource_activities::*, ApplicationError};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
    identity::UserId,
};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresResourceActivityStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresResourceActivityStore {
    pub fn open(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(url)?),
            hasher,
            clock,
        })
    }
    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| ApplicationError::Port("resource activity database lock poisoned".into()))
    }
}
impl ResourceActivityStore for PostgresResourceActivityStore {
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        query: ResourceActivityQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityPage, ApplicationError> {
        self.list_page(actor, case, resource, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        revision: Option<ResourceActivityRevision>,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityView, ApplicationError> {
        self.get_detail(actor, case, resource, id, revision, at)
    }
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        query: ResourceActivityHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityHistoryPage, ApplicationError> {
        self.history_page(actor, case, resource, id, query, at)
    }
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceActivityCommand,
    ) -> Result<ResourceActivityPreparation, ApplicationError> {
        self.prepare_change(actor, case, resource, command)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceActivityChange,
    ) -> Result<ResourceActivityDetail, ApplicationError> {
        self.commit_change(actor, case, resource, prepared)
    }
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return if error.as_db_error().and_then(|e| e.constraint())
            == Some("resource_activity_operation")
        {
            ResourceActivityError::OperationConflict.into()
        } else {
            ResourceActivityError::RevisionConflict.into()
        };
    }
    crate::postgres_port::error("resource activity database", error)
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    ResourceActivityError::StoredInconsistent(error.to_string()).into()
}
fn stored_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::Port(_) => error,
        _ => inconsistent(error),
    }
}
