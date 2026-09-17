//! Transactional hearing scheduling with exact history and explicit operation receipts.

mod authorization;
mod commit;
mod decode;
mod port_impl;
mod preparation;
mod query;
mod sources;
pub(crate) use sources::{administration, participant};
pub(crate) mod storage;
mod write;

use application::{hearings::*, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresHearingStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresHearingStore {
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
            .map_err(|_| ApplicationError::Port("hearing database lock poisoned".into()))
    }
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return if error.as_db_error().and_then(|e| e.constraint())
            == Some("hearing_operation_unique")
        {
            HearingError::OperationConflict.into()
        } else {
            HearingError::RevisionConflict.into()
        };
    }
    ApplicationError::Port(format!("hearing database: {error}"))
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    HearingError::StoredInconsistent(error.to_string()).into()
}
