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
use std::sync::{Arc, Mutex};

pub struct PostgresDeadlineDispatchStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PostgresDeadlineDispatchStore {
    pub fn open(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        let mut client = crate::postgres::open(url)?;
        // Set budgets before begin_audited attempts its first advisory lock.
        client
            .batch_execute("SET lock_timeout='1s'; SET statement_timeout='5s'")
            .map_err(port)?;
        Ok(Self {
            client: Mutex::new(client),
            hasher,
            clock,
        })
    }
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
    ApplicationError::Port(format!("deadline dispatch database: {error}"))
}

fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineError::StoredInconsistent(error.to_string()).into()
}
