//! Serialized, recoverable consumption of immutable deadline reevaluation jobs.
pub(crate) mod attempts_read;
mod attempts_write;
mod execute;
mod read_support;
pub(crate) mod results;
mod results_no_change;
mod results_write;
mod retry;
mod selection;

use application::{deadline_worker::*, deadlines::DeadlineError, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher, typed_participants::Uuid};
use postgres::{Client, Transaction};
use std::sync::{Arc, Mutex, MutexGuard};
use time::{OffsetDateTime, UtcOffset};

pub struct PostgresDeadlineWorkerStore {
    client: Mutex<Client>,
    url: String,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PostgresDeadlineWorkerStore {
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
            .map_err(|_| inconsistent("worker database lock poisoned"))?;
        self.reconnect(&mut client)?;
        Ok(client)
    }

    fn reconnect(&self, client: &mut Client) -> Result<(), ApplicationError> {
        if client.is_closed() {
            *client = connect(&self.url)?;
        }
        Ok(())
    }

    fn read<T>(
        &self,
        load: impl FnOnce(&mut Transaction<'_>, &dyn DocumentHasher) -> Result<T, ApplicationError>,
    ) -> Result<T, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let value = load(&mut tx, self.hasher.as_ref())?;
        tx.commit().map_err(port)?;
        Ok(value)
    }
}

impl DeadlineWorkerStore for PostgresDeadlineWorkerStore {
    fn run_next(&self) -> Result<DeadlineWorkerRun, ApplicationError> {
        let mut client = self.client()?;
        let at = clock_time(self.clock.as_ref())?;
        let mut context = execute::FailureContext::default();
        match execute::run(&mut client, self.hasher.as_ref(), at, &mut context) {
            Ok(value) => Ok(value),
            Err(error) => {
                let Some(target) = context.target else {
                    return Err(error);
                };
                self.reconnect(&mut client)?;
                let attempt_id = Uuid::new_v4();
                let failed_at = clock_time(self.clock.as_ref())?;
                match retry::record(
                    &mut client,
                    self.hasher.as_ref(),
                    target,
                    &context,
                    &error,
                    attempt_id,
                    failed_at,
                ) {
                    Ok(value) => Ok(value),
                    Err(recording_error) => {
                        // Reconcile this exact attempted write before reporting an uncertain reply.
                        self.reconnect(&mut client)?;
                        match retry::reconcile(
                            &mut client,
                            self.hasher.as_ref(),
                            target.id,
                            attempt_id,
                        )? {
                            Some(value) => Ok(value),
                            None => Err(recording_error),
                        }
                    }
                }
            }
        }
    }

    fn result(&self, job_id: Uuid) -> Result<Option<DeadlineWorkerResult>, ApplicationError> {
        self.read(|tx, hasher| results::load(tx, job_id, hasher))
    }

    fn latest_attempt(
        &self,
        job_id: Uuid,
    ) -> Result<Option<DeadlineWorkerAttempt>, ApplicationError> {
        self.read(|tx, hasher| attempts_read::latest(tx, job_id, hasher))
    }
}

fn connect(url: &str) -> Result<Client, ApplicationError> {
    let mut client = crate::postgres::open(url)?;
    // Startup inventory has its own cost; these budgets precede every worker transaction.
    client
        .batch_execute("SET lock_timeout='1s'; SET statement_timeout='5s'")
        .map_err(port)?;
    Ok(client)
}

fn clock_time(clock: &dyn Clock) -> Result<OffsetDateTime, ApplicationError> {
    let at = clock.now().to_offset(UtcOffset::UTC);
    if !(1..=9999).contains(&at.year()) {
        return Err(inconsistent("worker clock is outside supported years"));
    }
    Ok(at)
}

fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineError::StoredInconsistent(error.to_string()).into()
}

fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("deadline worker database", error)
}
