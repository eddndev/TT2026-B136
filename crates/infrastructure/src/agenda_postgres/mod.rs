//! Transactional combined agenda over authorized current resource heads.

mod authorization;
mod header;
mod projection;
mod query;

use application::ApplicationError;
use domain::{clock::Clock, crypto::DocumentHasher};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresAgendaStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PostgresAgendaStore {
    pub fn open(
        url: &str,
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

    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| inconsistent("agenda database lock poisoned"))
    }
}

fn port(error: Error) -> ApplicationError {
    crate::postgres_port::error("agenda database", error)
}

fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::Port(format!("stored agenda is inconsistent: {error}"))
}
