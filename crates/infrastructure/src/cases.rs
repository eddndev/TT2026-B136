//! PostgreSQL cases with immutable administration and current membership checks.

mod authorization;
mod mutation;
mod query;
mod storage;
mod values;

use application::cases::*;
use application::ApplicationError;
use domain::cases::{CaseId, CaseMetadata};
use domain::crypto::DocumentHasher;
use domain::identity::UserId;
use postgres::Client;
use std::sync::{Arc, Mutex, MutexGuard};
use time::OffsetDateTime;

pub struct PostgresCaseRepository {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
impl PostgresCaseRepository {
    pub fn connect(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::connect(url)?),
            hasher,
        })
    }
    pub fn open(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(url)?),
            hasher,
        })
    }
    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| ApplicationError::Port("case database lock poisoned".into()))
    }
}
impl CaseRepository for PostgresCaseRepository {
    fn create_basic(
        &self,
        actor: UserId,
        id: CaseId,
        metadata: CaseMetadata,
        at: OffsetDateTime,
    ) -> Result<CaseRecord, ApplicationError> {
        self.create(
            actor,
            id,
            CaseAdministrationValues::basic(metadata),
            false,
            at,
        )
        .map(|d| storage::basic(&d))
    }
    fn register_penal(
        &self,
        actor: UserId,
        id: CaseId,
        creation: PenalCaseCreation,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        self.create(actor, id, creation.into_values(), true, at)
    }
    fn list_basic(
        &self,
        actor: UserId,
        limit: u32,
        offset: u32,
        at: OffsetDateTime,
    ) -> Result<Vec<CaseRecord>, ApplicationError> {
        self.basic_list(actor, limit, offset, at)
    }
    fn get_basic(
        &self,
        actor: UserId,
        id: CaseId,
        at: OffsetDateTime,
    ) -> Result<CaseRecord, ApplicationError> {
        self.read(actor, id, false, at).map(|d| storage::basic(&d))
    }
    fn add_member(
        &self,
        id: CaseId,
        user_id: UserId,
        actor: UserId,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        self.member(id, user_id, actor, at, true)
    }
    fn remove_member(
        &self,
        id: CaseId,
        user_id: UserId,
        actor: UserId,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        self.member(id, user_id, actor, at, false)
    }
    fn replace_administration(
        &self,
        actor: UserId,
        id: CaseId,
        expected: CaseRevisionExpectation,
        values: CaseEditableValues,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        self.mutate(
            actor,
            id,
            expected,
            mutation::Change::Values(Box::new(values)),
            at,
        )
    }
    fn change_administrative_status(
        &self,
        actor: UserId,
        id: CaseId,
        expected: CaseRevisionExpectation,
        status: CaseAdministrativeStatus,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        self.mutate(actor, id, expected, mutation::Change::Status(status), at)
    }
    fn list_administrations(
        &self,
        actor: UserId,
        query: CaseAdministrationQuery,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationPage, ApplicationError> {
        self.administrative_list(actor, query, at)
    }
    fn get_administration(
        &self,
        actor: UserId,
        id: CaseId,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        self.read(actor, id, true, at)
    }
    fn administration_history(
        &self,
        actor: UserId,
        id: CaseId,
        query: CaseAdministrationHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationHistoryPage, ApplicationError> {
        self.history(actor, id, query, at)
    }
}
