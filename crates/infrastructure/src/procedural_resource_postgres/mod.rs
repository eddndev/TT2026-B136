//! Audited immutable procedural resources and declared acts.
mod authorization;
mod commit;
mod decode;
mod preparation;
mod query;
mod sources;
pub(crate) mod storage;
mod write;
use application::{documents::StageSupportReadLimits, procedural_resources::*, ApplicationError};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
    identity::UserId,
};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresProceduralResourceStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresProceduralResourceStore {
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
        self.client.lock().map_err(|_| {
            ApplicationError::Port("procedural resource database lock poisoned".into())
        })
    }
}
impl ProceduralResourceStore for PostgresProceduralResourceStore {
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        query: ResourceQuery,
        at: OffsetDateTime,
    ) -> Result<ResourcePage, ApplicationError> {
        self.list_page(actor, case, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        id: ResourceId,
        revision: Option<ResourceRevision>,
        at: OffsetDateTime,
    ) -> Result<ResourceDetail, ApplicationError> {
        self.get_detail(actor, case, id, revision, at)
    }
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        id: ResourceId,
        query: ResourceHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceHistoryPage, ApplicationError> {
        self.history_page(actor, case, id, query, at)
    }
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &ResourceCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<ResourcePreparation, ApplicationError> {
        self.prepare_change(actor, case, command, limits)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedResourceChange,
    ) -> Result<ResourceDetail, ApplicationError> {
        self.commit_change(actor, case, prepared)
    }
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return if error.as_db_error().and_then(|e| e.constraint())
            == Some("procedural_resource_operation_unique")
        {
            ProceduralResourceError::OperationConflict.into()
        } else {
            ProceduralResourceError::RevisionConflict.into()
        };
    }
    crate::postgres_port::error("procedural resource database", error)
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    ProceduralResourceError::StoredInconsistent(error.to_string()).into()
}
