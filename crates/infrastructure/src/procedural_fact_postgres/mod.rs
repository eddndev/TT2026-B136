//! Audited declarations with immutable exact historical sources.
pub(crate) mod administration;
mod authorization;
mod commit;
mod decode;
mod port_impl;
mod preparation;
mod query;
pub(crate) mod sources;
pub(crate) mod storage;
mod target;
mod write;
use application::{procedural_facts::*, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresProceduralFactStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresProceduralFactStore {
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
            .map_err(|_| ApplicationError::Port("procedural fact database lock poisoned".into()))
    }
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return if error.as_db_error().and_then(|e| e.constraint())
            == Some("procedural_fact_operation_unique")
        {
            ProceduralFactError::OperationConflict.into()
        } else {
            ProceduralFactError::RevisionConflict.into()
        };
    }
    crate::postgres_port::error("procedural fact database", error)
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    ProceduralFactError::StoredInconsistent(error.to_string()).into()
}
