//! Audited immutable precautionary appointment revisions and exact historical recovery.
mod audit;
mod authorization;
mod commit;
mod decode;
mod inventory;
mod preparation;
mod query;
mod sources;
mod storage;
mod write;
use application::{precautionary_hearings::PrecautionaryHearingError, ApplicationError};
use domain::{clock::Clock, crypto::DocumentHasher};
pub(crate) use inventory::validate_inventory;
use postgres::{Client, Error};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresPrecautionaryHearingStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresPrecautionaryHearingStore {
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
    fn now(&self) -> Result<time::OffsetDateTime, ApplicationError> {
        let at = self.clock.now();
        if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
            return Err(inconsistent("store clock must use supported UTC"));
        }
        Ok(at)
    }
    fn now_after(
        &self,
        floor: time::OffsetDateTime,
    ) -> Result<time::OffsetDateTime, ApplicationError> {
        let at = self.now()?;
        if at < floor {
            return Err(inconsistent("store clock precedes the returned capture"));
        }
        Ok(at)
    }
    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| ApplicationError::Port("precautionary database lock poisoned".into()))
    }
}
fn port(error: Error) -> ApplicationError {
    if error.code() == Some(&postgres::error::SqlState::UNIQUE_VIOLATION) {
        return PrecautionaryHearingError::OperationConflict.into();
    }
    crate::postgres_port::error("precautionary hearing database", error)
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    PrecautionaryHearingError::StoredInconsistent(error.to_string()).into()
}
