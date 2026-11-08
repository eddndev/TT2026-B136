//! Durable PostgreSQL user repository.

use std::str::FromStr;
use std::sync::Mutex;

use crate::audit_postgres::{append_transaction, begin_audited};
use application::identity::{
    ca_operator_display_name, UserRecord, UserRepository, CA_OPERATOR_DISPLAY_NAME,
};
use application::ApplicationError;
use domain::clock::OffsetDateTime;
use domain::crypto::RecoveryCodeSet;
use domain::identity::{DisplayName, Role, UserId};
use postgres::error::SqlState;
use postgres::{Client, Row};

const CA_USER_COLUMNS: &str = "id, email, password_hash, role, active, \
    protected_totp_secret, recovery_codes, revision, auth_generation";
const OFFICE_USER_COLUMNS: &str = "id, email, password_hash, role, active, \
    protected_totp_secret, recovery_codes, revision, auth_generation, display_name";

/// Column family of a user table.
///
/// Office accounts store a display name. The separately provisioned CA identity
/// store keeps none and reads a fixed label instead; see
/// docs/adr/0087-account-display-names.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserColumns {
    /// The office `users` table, which stores display names.
    Office,
    /// The `qadra_ca.users` table, which keeps no display names.
    CertificateAuthority,
}

impl UserColumns {
    fn select(self) -> &'static str {
        match self {
            Self::Office => OFFICE_USER_COLUMNS,
            Self::CertificateAuthority => CA_USER_COLUMNS,
        }
    }
}

/// Durable user repository backed by PostgreSQL.
pub struct PostgresUserRepository {
    client: Mutex<Client>,
    columns: UserColumns,
}

impl PostgresUserRepository {
    /// Connects and applies idempotent schema migrations.
    pub fn connect(database_url: &str) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::connect(database_url)?),
            columns: UserColumns::Office,
        })
    }

    /// Opens the prepared schema with a restricted runtime role and no DDL.
    pub fn open(
        database_url: &(impl crate::PostgresConnectionSource + ?Sized),
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(database_url)?),
            columns: crate::postgres_source::user_columns(database_url),
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

    fn insert_initial_owner(
        &self,
        user: UserRecord,
        at: OffsetDateTime,
    ) -> Result<bool, ApplicationError> {
        let mut client = self.lock()?;
        let mut transaction = begin_audited(&mut client)?;
        if transaction
            .query_opt("SELECT id FROM users LIMIT 1", &[])
            .map_err(port_error)?
            .is_some()
        {
            transaction.rollback().map_err(port_error)?;
            return Ok(false);
        }
        if user.role != Role::Owner || !user.active {
            return Err(ApplicationError::PermissionDenied);
        }
        insert_user(&mut transaction, &user, self.columns)?;
        append_transaction(
            &mut transaction,
            "system",
            "identity.owner_bootstrapped",
            &user.email,
            at,
        )?;
        transaction.commit().map_err(port_error)?;
        Ok(true)
    }

    fn insert(
        &self,
        user: UserRecord,
        actor: UserId,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        let mut client = self.lock()?;
        let mut transaction = begin_audited(&mut client)?;
        let row = transaction
            .query_opt(
                "SELECT email FROM users WHERE id = $1 AND active AND role = 'owner' FOR SHARE",
                &[&actor.as_uuid()],
            )
            .map_err(port_error)?
            .ok_or(ApplicationError::PermissionDenied)?;
        let email: String = row.get(0);
        insert_user(&mut transaction, &user, self.columns)?;
        append_transaction(
            &mut transaction,
            &email,
            "identity.user_created",
            &user.email,
            at,
        )?;
        transaction.commit().map_err(port_error)
    }

    fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, ApplicationError> {
        let sql = format!(
            "SELECT {} FROM users WHERE email = $1",
            self.columns.select()
        );
        self.lock()?
            .query_opt(&sql, &[&email])
            .map_err(port_error)?
            .map(|row| row_to_user(row, self.columns))
            .transpose()
    }

    fn find_by_id(&self, id: UserId) -> Result<Option<UserRecord>, ApplicationError> {
        let sql = format!("SELECT {} FROM users WHERE id = $1", self.columns.select());
        self.lock()?
            .query_opt(&sql, &[&id.as_uuid()])
            .map_err(port_error)?
            .map(|row| row_to_user(row, self.columns))
            .transpose()
    }

    fn replace_recovery_codes(
        &self,
        id: UserId,
        expected_revision: u64,
        codes: RecoveryCodeSet,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        let json = serde_json::to_value(codes).map_err(|error| {
            ApplicationError::Port(format!("recovery code serialization failed: {error}"))
        })?;
        let expected = i64::try_from(expected_revision)
            .map_err(|_| ApplicationError::Port("user revision exceeds i64".into()))?;
        let mut client = self.lock()?;
        let mut transaction = begin_audited(&mut client)?;
        let row = transaction
            .query_opt(
                "UPDATE users SET recovery_codes = $1, revision = revision + 1,
             updated_at = CURRENT_TIMESTAMP WHERE id = $2 AND revision = $3 AND active
             RETURNING email",
                &[&json, &id.as_uuid(), &expected],
            )
            .map_err(port_error)?
            .ok_or(ApplicationError::ConcurrentModification)?;
        let email: String = row.get(0);
        append_transaction(
            &mut transaction,
            &email,
            "identity.recovery_consumed",
            &id.to_string(),
            at,
        )?;
        transaction.commit().map_err(port_error)
    }
}

pub(crate) fn insert_user<C>(
    client: &mut C,
    user: &UserRecord,
    columns: UserColumns,
) -> Result<(), ApplicationError>
where
    C: postgres::GenericClient,
{
    let recovery_codes = serde_json::to_value(&user.recovery_codes).map_err(|error| {
        ApplicationError::Port(format!("recovery code serialization failed: {error}"))
    })?;
    if user.auth_generation != 0 {
        return Err(ApplicationError::InvalidInput(
            "new accounts require generation zero".into(),
        ));
    }
    let revision = i64::try_from(user.revision)
        .map_err(|_| ApplicationError::Port("user revision exceeds i64".to_string()))?;
    let id = user.id.as_uuid();
    let role = user.role.as_str();
    let name = user.display_name.as_str();
    let mut values: Vec<&(dyn postgres::types::ToSql + Sync)> = vec![
        &id,
        &user.email,
        &user.password_hash,
        &role,
        &user.active,
        &user.protected_totp_secret,
        &recovery_codes,
        &revision,
    ];
    let sql = match columns {
        UserColumns::Office => {
            values.push(&name);
            "INSERT INTO users \
             (id, email, password_hash, role, active, protected_totp_secret, \
              recovery_codes, revision, display_name) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        }
        UserColumns::CertificateAuthority => {
            if name != CA_OPERATOR_DISPLAY_NAME {
                return Err(ApplicationError::InvalidInput(
                    "CA operator records keep no display name".into(),
                ));
            }
            "INSERT INTO users \
             (id, email, password_hash, role, active, protected_totp_secret, \
              recovery_codes, revision) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        }
    };
    client
        .execute(sql, &values)
        .map(|_| ())
        .map_err(map_insert_error)
}

fn row_to_user(row: Row, columns: UserColumns) -> Result<UserRecord, ApplicationError> {
    let role_name: String = row.get("role");
    let recovery_json: serde_json::Value = row.get("recovery_codes");
    let revision: i64 = row.get("revision");
    Ok(UserRecord {
        id: UserId::from_uuid(row.get("id")),
        email: row.get("email"),
        display_name: match columns {
            UserColumns::Office => DisplayName::from_stored(row.get("display_name"))
                .map_err(|_| ApplicationError::Port("stored display name is invalid".into()))?,
            UserColumns::CertificateAuthority => ca_operator_display_name(),
        },
        password_hash: row.get("password_hash"),
        role: Role::from_str(&role_name)?,
        active: row.get("active"),
        auth_generation: u64::try_from(row.get::<_, i64>("auth_generation")).map_err(|_| {
            ApplicationError::Port("stored authentication generation is negative".into())
        })?,
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
