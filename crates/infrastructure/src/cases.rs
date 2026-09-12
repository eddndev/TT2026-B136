//! PostgreSQL case metadata and membership persistence.

use std::sync::{Mutex, MutexGuard};

use application::cases::{CaseAccess, CaseRecord, CaseRepository};
use application::ApplicationError;
use domain::cases::{CaseId, CaseMetadata};
use domain::identity::UserId;
use postgres::{Client, Row, Transaction};
use uuid::Uuid;

/// Persists case creation and the creator's membership in one transaction.
pub struct PostgresCaseRepository {
    client: Mutex<Client>,
}

impl PostgresCaseRepository {
    /// Initializes identity prerequisites and case tables before serving queries.
    pub fn connect(database_url: &str) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::connect(database_url)?),
        })
    }

    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| ApplicationError::Port("case database lock poisoned".into()))
    }
}

impl CaseRepository for PostgresCaseRepository {
    fn insert(&self, record: CaseRecord) -> Result<(), ApplicationError> {
        let metadata = CaseMetadata::new(&record.title, &record.reference)?;
        let mut client = self.client()?;
        let mut transaction = client.transaction().map_err(port_error)?;
        require_active_user(&mut transaction, record.created_by)?;
        transaction
            .execute(
                "INSERT INTO cases (id, title, reference, created_by) VALUES ($1, $2, $3, $4)",
                &[
                    &record.id.as_uuid(),
                    &metadata.title(),
                    &metadata.reference(),
                    &record.created_by.as_uuid(),
                ],
            )
            .map_err(port_error)?;
        transaction
            .execute(
                "INSERT INTO case_memberships (case_id, user_id) VALUES ($1, $2)",
                &[&record.id.as_uuid(), &record.created_by.as_uuid()],
            )
            .map_err(port_error)?;
        transaction.commit().map_err(port_error)
    }

    fn list(
        &self,
        access: CaseAccess,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<CaseRecord>, ApplicationError> {
        let member = assigned_user(access);
        let rows = self
            .client()?
            .query(
                "SELECT id, title, reference, created_by FROM cases c
                 WHERE $1::uuid IS NULL OR EXISTS (
                     SELECT 1 FROM case_memberships m WHERE m.case_id = c.id AND m.user_id = $1
                 ) ORDER BY c.id LIMIT $2 OFFSET $3",
                &[&member, &i64::from(limit), &i64::from(offset)],
            )
            .map_err(port_error)?;
        Ok(rows.into_iter().map(row_to_case).collect())
    }

    fn find(&self, id: CaseId, access: CaseAccess) -> Result<Option<CaseRecord>, ApplicationError> {
        let member = assigned_user(access);
        self.client()?
            .query_opt(
                "SELECT id, title, reference, created_by FROM cases c
                 WHERE c.id = $1 AND ($2::uuid IS NULL OR EXISTS (
                     SELECT 1 FROM case_memberships m WHERE m.case_id = c.id AND m.user_id = $2
                 ))",
                &[&id.as_uuid(), &member],
            )
            .map(|row| row.map(row_to_case))
            .map_err(port_error)
    }

    fn add_member(&self, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = client.transaction().map_err(port_error)?;
        require_case(&mut transaction, id)?;
        require_active_user(&mut transaction, user_id)?;
        transaction
            .execute(
                "INSERT INTO case_memberships (case_id, user_id) VALUES ($1, $2)
                 ON CONFLICT (case_id, user_id) DO NOTHING",
                &[&id.as_uuid(), &user_id.as_uuid()],
            )
            .map_err(port_error)?;
        transaction.commit().map_err(port_error)
    }

    fn remove_member(&self, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = client.transaction().map_err(port_error)?;
        require_case(&mut transaction, id)?;
        transaction
            .execute(
                "DELETE FROM case_memberships WHERE case_id = $1 AND user_id = $2",
                &[&id.as_uuid(), &user_id.as_uuid()],
            )
            .map_err(port_error)?;
        transaction.commit().map_err(port_error)
    }
}

fn assigned_user(access: CaseAccess) -> Option<Uuid> {
    match access {
        CaseAccess::All => None,
        CaseAccess::Assigned(id) => Some(id.as_uuid()),
    }
}

fn require_case(transaction: &mut Transaction<'_>, id: CaseId) -> Result<(), ApplicationError> {
    transaction
        .query_opt(
            "SELECT id FROM cases WHERE id = $1 FOR SHARE",
            &[&id.as_uuid()],
        )
        .map_err(port_error)?
        .ok_or(ApplicationError::CaseNotFound)?;
    Ok(())
}

fn require_active_user(
    transaction: &mut Transaction<'_>,
    id: UserId,
) -> Result<(), ApplicationError> {
    transaction
        .query_opt(
            "SELECT id FROM users WHERE id = $1 AND active FOR SHARE",
            &[&id.as_uuid()],
        )
        .map_err(port_error)?
        .ok_or(ApplicationError::UserNotFound)?;
    Ok(())
}

fn row_to_case(row: Row) -> CaseRecord {
    CaseRecord {
        id: CaseId::from_uuid(row.get("id")),
        title: row.get("title"),
        reference: row.get("reference"),
        created_by: UserId::from_uuid(row.get("created_by")),
    }
}

fn port_error(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("case database: {error}"))
}
