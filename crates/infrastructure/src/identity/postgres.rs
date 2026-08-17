//! PostgreSQL user repository and schema migration runner.

use std::str::FromStr;
use std::sync::Mutex;

use application::identity::{UserRecord, UserRepository};
use application::ApplicationError;
use domain::crypto::RecoveryCodeSet;
use domain::identity::{Role, UserId};
use postgres::error::SqlState;
use postgres::{Client, NoTls, Row};

const MIGRATION: &str = include_str!("../../../../migrations/0001_identity.sql");
const USER_COLUMNS: &str = "id, email, password_hash, role, active, \
    protected_totp_secret, recovery_codes, revision";

/// Durable user repository backed by PostgreSQL.
pub struct PostgresUserRepository {
    client: Mutex<Client>,
}

impl PostgresUserRepository {
    /// Connects and applies idempotent schema migrations.
    pub fn connect(database_url: &str) -> Result<Self, ApplicationError> {
        let mut client = Client::connect(database_url, NoTls).map_err(port_error)?;
        client.batch_execute(MIGRATION).map_err(port_error)?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| ApplicationError::Port("postgres client lock poisoned".to_string()))
    }
}

impl UserRepository for PostgresUserRepository {
    fn has_users(&self) -> Result<bool, ApplicationError> {
        self.lock()?
            .query_one("SELECT EXISTS(SELECT 1 FROM users)", &[])
            .map(|row| row.get(0))
            .map_err(port_error)
    }

    fn insert_initial_owner(&self, user: UserRecord) -> Result<bool, ApplicationError> {
        let mut client = self.lock()?;
        let mut transaction = client.transaction().map_err(port_error)?;
        transaction
            .batch_execute("LOCK TABLE users IN EXCLUSIVE MODE")
            .map_err(port_error)?;
        if transaction
            .query_opt("SELECT id FROM users LIMIT 1", &[])
            .map_err(port_error)?
            .is_some()
        {
            transaction.rollback().map_err(port_error)?;
            return Ok(false);
        }
        insert_user(&mut transaction, &user)?;
        transaction.commit().map_err(port_error)?;
        Ok(true)
    }

    fn insert(&self, user: UserRecord) -> Result<(), ApplicationError> {
        insert_user(&mut *self.lock()?, &user)
    }

    fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, ApplicationError> {
        let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE email = $1");
        self.lock()?
            .query_opt(&sql, &[&email])
            .map_err(port_error)?
            .map(row_to_user)
            .transpose()
    }

    fn find_by_id(&self, id: UserId) -> Result<Option<UserRecord>, ApplicationError> {
        let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1");
        self.lock()?
            .query_opt(&sql, &[&id.as_uuid()])
            .map_err(port_error)?
            .map(row_to_user)
            .transpose()
    }

    fn replace_recovery_codes(
        &self,
        id: UserId,
        expected_revision: u64,
        codes: RecoveryCodeSet,
    ) -> Result<(), ApplicationError> {
        let json = serde_json::to_value(codes).map_err(|error| {
            ApplicationError::Port(format!("recovery code serialization failed: {error}"))
        })?;
        let expected = i64::try_from(expected_revision)
            .map_err(|_| ApplicationError::Port("user revision exceeds i64".to_string()))?;
        let changed = self
            .lock()?
            .execute(
                "UPDATE users SET recovery_codes = $1, revision = revision + 1, \
                 updated_at = CURRENT_TIMESTAMP WHERE id = $2 AND revision = $3",
                &[&json, &id.as_uuid(), &expected],
            )
            .map_err(port_error)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(ApplicationError::ConcurrentModification)
        }
    }
}

fn insert_user<C>(client: &mut C, user: &UserRecord) -> Result<(), ApplicationError>
where
    C: postgres::GenericClient,
{
    let recovery_codes = serde_json::to_value(&user.recovery_codes).map_err(|error| {
        ApplicationError::Port(format!("recovery code serialization failed: {error}"))
    })?;
    let revision = i64::try_from(user.revision)
        .map_err(|_| ApplicationError::Port("user revision exceeds i64".to_string()))?;
    client
        .execute(
            "INSERT INTO users \
             (id, email, password_hash, role, active, protected_totp_secret, \
              recovery_codes, revision) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[
                &user.id.as_uuid(),
                &user.email,
                &user.password_hash,
                &user.role.as_str(),
                &user.active,
                &user.protected_totp_secret,
                &recovery_codes,
                &revision,
            ],
        )
        .map(|_| ())
        .map_err(map_insert_error)
}

fn row_to_user(row: Row) -> Result<UserRecord, ApplicationError> {
    let role_name: String = row.get("role");
    let recovery_json: serde_json::Value = row.get("recovery_codes");
    let revision: i64 = row.get("revision");
    Ok(UserRecord {
        id: UserId::from_uuid(row.get("id")),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        role: Role::from_str(&role_name)?,
        active: row.get("active"),
        protected_totp_secret: row.get("protected_totp_secret"),
        recovery_codes: serde_json::from_value(recovery_json).map_err(|error| {
            ApplicationError::Port(format!("stored recovery codes are invalid: {error}"))
        })?,
        revision: u64::try_from(revision)
            .map_err(|_| ApplicationError::Port("stored user revision is negative".to_string()))?,
    })
}

fn map_insert_error(error: postgres::Error) -> ApplicationError {
    if error.code() == Some(&SqlState::UNIQUE_VIOLATION) {
        ApplicationError::UserAlreadyExists
    } else {
        port_error(error)
    }
}

fn port_error(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("postgres: {error}"))
}
