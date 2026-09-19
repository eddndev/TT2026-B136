//! Audited, bounded expansion of source events and legacy deadline records.
mod cursor;
mod dispatch;
mod events;
pub(crate) use events::decode as decode_source_event;
mod jobs;
mod selection;

use application::{deadlines::DeadlineError, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresDeadlineDispatchStore {
    client: Mutex<Client>,
    url: String,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PostgresDeadlineDispatchStore {
    pub fn open(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(connect(url)?),
            url: url.to_owned(),
            hasher,
            clock,
        })
    }

    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| inconsistent("dispatch database lock poisoned"))?;
        if client.is_closed() {
            *client = connect(&self.url)?;
        }
        Ok(client)
    }
}

fn connect(url: &str) -> Result<Client, ApplicationError> {
    let mut client = crate::postgres::open(url)?;
    // Reconnection must validate the inventory and restore transaction budgets.
    client
        .batch_execute("SET lock_timeout='1s'; SET statement_timeout='5s'")
        .map_err(port)?;
    Ok(client)
}

fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION)
        && matches!(
            error.as_db_error().and_then(|e| e.constraint()),
            Some("deadline_job_operation" | "deadline_operation_unique")
        )
    {
        return DeadlineError::OperationConflict.into();
    }
    crate::postgres_port::error("deadline dispatch database", error)
}

fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineError::StoredInconsistent(error.to_string()).into()
}
