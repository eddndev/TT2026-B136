//! Audited global and case-specific rule definitions with exact historical evidence.
mod authorization;
mod commit;
mod decode;
mod header;
mod port_impl;
mod preparation;
mod projection;
mod query;
pub(crate) mod storage;
mod summary;
mod write;
use application::{deadline_profiles::*, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresDeadlineProfileStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresDeadlineProfileStore {
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
            .map_err(|_| ApplicationError::Port("profile database lock poisoned".into()))
    }
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return if error.as_db_error().and_then(|e| e.constraint())
            == Some("deadline_profile_operation_unique")
        {
            DeadlineProfileError::OperationConflict.into()
        } else {
            DeadlineProfileError::RevisionConflict.into()
        };
    }
    crate::postgres_port::error("profile database", error)
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineProfileError::StoredInconsistent(error.to_string()).into()
}
