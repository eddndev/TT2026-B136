//! Shared PostgreSQL connection and schema initialization boundary.

use application::ApplicationError;
use postgres::{Client, NoTls};

const IDENTITY_MIGRATION: &str = include_str!("../../../migrations/0001_identity.sql");
const CASE_MIGRATION: &str = include_str!("../../../migrations/0002_cases.sql");
// Every adapter uses this same database-scoped lock before applying schema DDL.
const SCHEMA_MIGRATION_LOCK: i64 = 0x4341534553;

/// Applies all schema prerequisites atomically, serialized across processes.
pub(crate) fn connect(database_url: &str) -> Result<Client, ApplicationError> {
    let mut client = Client::connect(database_url, NoTls).map_err(port_error)?;
    let mut transaction = client.transaction().map_err(port_error)?;
    transaction
        .query_one(
            "SELECT pg_advisory_xact_lock($1)",
            &[&SCHEMA_MIGRATION_LOCK],
        )
        .map_err(port_error)?;
    transaction
        .batch_execute(IDENTITY_MIGRATION)
        .map_err(port_error)?;
    transaction
        .batch_execute(CASE_MIGRATION)
        .map_err(port_error)?;
    transaction.commit().map_err(port_error)?;
    Ok(client)
}

fn port_error(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("postgres initialization: {error}"))
}
