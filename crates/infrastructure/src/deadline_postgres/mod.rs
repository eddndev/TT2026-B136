//! Immutable deadline evaluations and attention in one audited transaction.
mod administration;
mod attention;
mod authorization;
mod commit;
mod decode;
mod dependencies;
mod header;
#[cfg(test)]
mod legacy_tests;
mod port_impl;
mod preparation;
mod projection;
mod query;
mod responsibles;
pub(crate) mod storage;
mod tracking;
mod write;
use application::{deadlines::DeadlineError, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresDeadlineStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresDeadlineStore {
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
            .map_err(|_| ApplicationError::Port("deadline database lock poisoned".into()))
    }
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return if matches!(
            error.as_db_error().and_then(|e| e.constraint()),
            Some("deadline_operation_unique" | "deadline_job_operation")
        ) {
            DeadlineError::OperationConflict.into()
        } else {
            DeadlineError::RevisionConflict.into()
        };
    }
    ApplicationError::Port(format!("deadline database: {error}"))
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineError::StoredInconsistent(error.to_string()).into()
}
