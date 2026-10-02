//! Complete authorized dashboard snapshots in an audited transaction.

mod cases;
mod deadlines;
mod documents;
mod query;
mod workload;

use application::ApplicationError;
use domain::{clock::Clock, crypto::DocumentHasher};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex};

const MAX_CASES: usize = 1000;
const MAX_DOCUMENTS: usize = 10_000;
const MAX_DEADLINES: usize = 1000;
const MAX_WORKLOAD: usize = 1000;

pub struct PostgresDashboardStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresDashboardStore {
    pub fn open(
        url: &(impl crate::PostgresConnectionSource + ?Sized),
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        let mut client = crate::postgres::open(url)?;
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
    crate::postgres_port::error("dashboard database", error)
}
fn inconsistent(message: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::Port(format!("dashboard snapshot is inconsistent: {message}"))
}
fn capacity(actual: usize, maximum: usize, resource: &str) -> Result<(), ApplicationError> {
    if actual > maximum {
        return Err(ApplicationError::Port(format!(
            "dashboard {resource} capacity exceeded"
        )));
    }
    Ok(())
}
