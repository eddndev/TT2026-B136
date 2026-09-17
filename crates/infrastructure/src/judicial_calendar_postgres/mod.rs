//! Audited global calendar revisions with immutable civil classifications.
mod authorization;
mod commit;
mod decode;
mod port_impl;
mod preparation;
mod projection;
mod query;
pub(crate) mod storage;
mod write;
use application::{judicial_calendars::*, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresJudicialCalendarStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresJudicialCalendarStore {
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
            .map_err(|_| ApplicationError::Port("calendar database lock poisoned".into()))
    }
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return if error.as_db_error().and_then(|e| e.constraint())
            == Some("judicial_calendar_operation_unique")
        {
            JudicialCalendarError::OperationConflict.into()
        } else {
            JudicialCalendarError::RevisionConflict.into()
        };
    }
    ApplicationError::Port(format!("calendar database: {error}"))
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    JudicialCalendarError::StoredInconsistent(error.to_string()).into()
}
