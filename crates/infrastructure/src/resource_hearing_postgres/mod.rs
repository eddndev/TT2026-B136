//! Audited atomic resource appointment and its original association.
mod commit;
mod inventory;
mod preparation;
mod replay;
pub(crate) mod storage;
mod write;
use application::{
    resource_activities::ResourceActivityError, resource_hearings::*, ApplicationError,
};
use domain::{
    cases::CaseId, clock::Clock, crypto::DocumentHasher, identity::UserId,
    procedural_resources::ResourceId,
};
pub(crate) use inventory::validate_inventory;
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresResourceHearingStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresResourceHearingStore {
    pub fn open(
        url: &(impl crate::PostgresConnectionSource + ?Sized),
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
            .map_err(|_| ApplicationError::Port("resource hearing database lock poisoned".into()))
    }
}
impl ResourceHearingStore for PostgresResourceHearingStore {
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceHearingCommand,
    ) -> Result<ResourceHearingPreparation, ApplicationError> {
        self.prepare_change(actor, case, resource, command)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceHearing,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        self.commit_change(actor, case, resource, prepared)
    }
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return ResourceActivityError::OperationConflict.into();
    }
    crate::postgres_port::error("resource hearing database", error)
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    ResourceActivityError::StoredInconsistent(error.to_string()).into()
}
