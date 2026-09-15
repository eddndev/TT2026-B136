//! Audited PostgreSQL directory, separate from account access assignments.

mod authorization;
mod query;
mod storage;

use std::sync::{Arc, Mutex, MutexGuard};

use application::participants::{
    DirectoryStatus, ParticipantAction, ParticipantHistoryPage, ParticipantHistoryQuery,
    ParticipantId, ParticipantPage, ParticipantQuery, ParticipantRevision, ParticipantSnapshot,
    ParticipantStore, ParticipantValues,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::DocumentHasher;
use domain::identity::UserId;
use postgres::Client;
use time::OffsetDateTime;

use crate::audit_postgres::{append_transaction, begin_audited};
use authorization::authorize;
use storage::port;

pub struct PostgresParticipantStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}

impl PostgresParticipantStore {
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
            .map_err(|_| ApplicationError::Port("participant database lock poisoned".into()))
    }

    #[allow(clippy::too_many_arguments)]
    fn mutate(
        &self,
        actor: UserId,
        case: CaseId,
        id: ParticipantId,
        expected: ParticipantRevision,
        change: Change,
        at: OffsetDateTime,
    ) -> Result<ParticipantSnapshot, ApplicationError> {
        let action = match &change {
            Change::Values(_) => ParticipantAction::Replace,
            Change::Status(_) => ParticipantAction::ChangeStatus,
        };
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal = authorize(&mut transaction, actor, case, action, true)?;
        let current = storage::current(&mut transaction, case, id, self.hasher.as_ref())?;
        if current.revision != expected {
            return Err(ApplicationError::ParticipantRevisionConflict);
        }
        let revision = expected
            .next()
            .ok_or(ApplicationError::ParticipantRevisionExhausted)?;
        let values = match change {
            Change::Values(values) => values,
            Change::Status(status) => current.values.with_directory_status(status),
        };
        let snapshot = storage::snapshot(
            case,
            id,
            revision,
            values,
            &principal,
            at,
            self.hasher.as_ref(),
        );
        storage::insert(&mut transaction, &snapshot)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            action.audit_action(),
            &storage::resource(&snapshot),
            at,
        )?;
        transaction.commit().map_err(port)?;
        Ok(snapshot)
    }
}

enum Change {
    Values(ParticipantValues),
    Status(DirectoryStatus),
}

impl ParticipantStore for PostgresParticipantStore {
    fn create(
        &self,
        actor: UserId,
        case: CaseId,
        id: ParticipantId,
        values: ParticipantValues,
        at: OffsetDateTime,
    ) -> Result<ParticipantSnapshot, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal = authorize(
            &mut transaction,
            actor,
            case,
            ParticipantAction::Create,
            false,
        )?;
        if values.directory_status() != DirectoryStatus::Active {
            return Err(ApplicationError::InvalidInput(
                "participant creation requires active directory status".into(),
            ));
        }
        transaction
            .execute(
                "INSERT INTO case_participants(id,case_id) VALUES($1,$2)",
                &[&id.as_uuid(), &case.as_uuid()],
            )
            .map_err(port)?;
        let snapshot = storage::snapshot(
            case,
            id,
            ParticipantRevision::initial(),
            values,
            &principal,
            at,
            self.hasher.as_ref(),
        );
        storage::insert(&mut transaction, &snapshot)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            ParticipantAction::Create.audit_action(),
            &storage::resource(&snapshot),
            at,
        )?;
        transaction.commit().map_err(port)?;
        Ok(snapshot)
    }

    fn replace(
        &self,
        actor: UserId,
        case: CaseId,
        id: ParticipantId,
        expected: ParticipantRevision,
        values: ParticipantValues,
        at: OffsetDateTime,
    ) -> Result<ParticipantSnapshot, ApplicationError> {
        self.mutate(actor, case, id, expected, Change::Values(values), at)
    }

    fn change_status(
        &self,
        actor: UserId,
        case: CaseId,
        id: ParticipantId,
        expected: ParticipantRevision,
        status: DirectoryStatus,
        at: OffsetDateTime,
    ) -> Result<ParticipantSnapshot, ApplicationError> {
        self.mutate(actor, case, id, expected, Change::Status(status), at)
    }

    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        id: ParticipantId,
        at: OffsetDateTime,
    ) -> Result<ParticipantSnapshot, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal = authorize(&mut transaction, actor, case, ParticipantAction::Read, true)?;
        let snapshot = storage::current(&mut transaction, case, id, self.hasher.as_ref())?;
        append_transaction(
            &mut transaction,
            &principal.email,
            ParticipantAction::Read.audit_action(),
            &storage::resource(&snapshot),
            at,
        )?;
        transaction.commit().map_err(port)?;
        Ok(snapshot)
    }

    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        query: ParticipantQuery,
        at: OffsetDateTime,
    ) -> Result<ParticipantPage, ApplicationError> {
        self.list_page(actor, case, query, at)
    }

    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        id: ParticipantId,
        query: ParticipantHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ParticipantHistoryPage, ApplicationError> {
        self.history_page(actor, case, id, query, at)
    }
}
