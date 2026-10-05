//! Atomic recording corrections and their retained judicial ancestry.
mod audit;
mod commit;
pub(crate) mod decode;
mod dependencies;
pub(crate) mod inventory;
mod preparation;
mod query;
pub(crate) mod storage;
mod write;

use crate::precautionary_hearing_postgres::authorization::authorize;
use application::{measure_corrections::MeasureAdministrativeError, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresMeasureAdministrativeStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PostgresMeasureAdministrativeStore {
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
            .map_err(|_| ApplicationError::Port("administrative measure lock poisoned".into()))
    }
    fn now(
        &self,
        floor: Option<time::OffsetDateTime>,
    ) -> Result<time::OffsetDateTime, ApplicationError> {
        let at = self.clock.now();
        if at.offset() != time::UtcOffset::UTC
            || !(1..=9999).contains(&at.year())
            || floor.is_some_and(|floor| at < floor)
        {
            return Err(inconsistent(
                "store clock is unsupported or precedes its capture",
            ));
        }
        Ok(at)
    }
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    MeasureAdministrativeError::StoredInconsistent(error.to_string()).into()
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return MeasureAdministrativeError::OperationConflict.into();
    }
    crate::postgres_port::error("administrative measure database", error)
}
