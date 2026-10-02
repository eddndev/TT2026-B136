//! Atomic contextual deadline registration using existing immutable tables.
mod commit;
mod preparation;
mod replay;
use application::{resource_activities::*, resource_deadlines::*, ApplicationError};
use domain::{cases::CaseId, clock::Clock, crypto::DocumentHasher, identity::UserId};
use postgres::Client;
use std::sync::{Arc, Mutex, MutexGuard};
pub struct PostgresResourceDeadlineStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresResourceDeadlineStore {
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
        self.client.lock().map_err(|_| {
            ApplicationError::Port("contextual deadline database lock poisoned".into())
        })
    }
}
impl ResourceDeadlineStore for PostgresResourceDeadlineStore {
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceDeadlineCommand,
    ) -> Result<ResourceDeadlinePreparation, ApplicationError> {
        self.prepare_change(actor, case, resource, command)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceDeadline,
    ) -> Result<ResourceDeadlineResult, ApplicationError> {
        self.commit_change(actor, case, resource, prepared)
    }
}
fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("contextual deadline database", error)
}
fn inconsistent(message: &str) -> ApplicationError {
    ResourceActivityError::StoredInconsistent(message.into()).into()
}
