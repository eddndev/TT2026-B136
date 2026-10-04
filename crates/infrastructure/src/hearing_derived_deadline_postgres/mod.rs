//! Atomic ordinary hearing result, configured deadline and immutable origin.
mod commit;
mod preparation;
mod replay;
mod write;

use application::{hearing_derived_deadlines::*, identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::DocumentHasher,
    identity::{Permission, UserId},
};
use postgres::{Client, Transaction};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct PostgresHearingDerivedDeadlineStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PostgresHearingDerivedDeadlineStore {
    pub fn open(
        url: &(impl crate::PostgresConnectionSource + ?Sized),
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
        self.client.lock().map_err(|_| {
            ApplicationError::Port("hearing consequence database lock poisoned".into())
        })
    }
}

fn authorize(
    tx: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
) -> Result<Principal, ApplicationError> {
    let principal =
        crate::hearing_result_postgres::authorization::authorize(tx, actor, case, true)?;
    if !principal.role.allows(Permission::ManageDeadline) {
        return Err(ApplicationError::PermissionDenied);
    }
    Ok(principal)
}

fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("hearing consequence database", error)
}
fn inconsistent(message: &str) -> ApplicationError {
    application::deadlines::DeadlineError::StoredInconsistent(message.into()).into()
}
