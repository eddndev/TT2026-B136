//! Atomic judicial decisions and their original measure ownership.
mod advertised;
mod anchors;
mod audit;
mod commit;
mod decode;
mod history;
mod history_budget;
mod inventory;
mod inventory_shape;
mod loaded_history;
mod preparation;
mod query;
mod sources;
mod storage;
mod write;
use crate::precautionary_hearing_postgres::authorization::authorize;
use application::{precautionary_measures::MeasureDecisionError, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
pub(crate) use history::load_measure_targets;
pub(crate) use inventory::validate_inventory;
pub(crate) use loaded_history::LoadedMeasureHistory;
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresMeasureDecisionStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresMeasureDecisionStore {
    pub fn open(
        source: &(impl crate::PostgresConnectionSource + ?Sized),
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(source)?),
            hasher,
            clock,
        })
    }
    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| ApplicationError::Port("measure database lock poisoned".into()))
    }
    fn now(
        &self,
        floor: Option<time::OffsetDateTime>,
    ) -> Result<time::OffsetDateTime, ApplicationError> {
        let at = self.clock.now();
        if at.offset() != time::UtcOffset::UTC
            || !(1..=9999).contains(&at.year())
            || floor.is_some_and(|f| at < f)
        {
            return Err(inconsistent(
                "store clock is unsupported or precedes the returned group",
            ));
        }
        Ok(at)
    }
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    MeasureDecisionError::StoredInconsistent(error.to_string()).into()
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return MeasureDecisionError::OperationConflict.into();
    }
    crate::postgres_port::error("measure decision database", error)
}
