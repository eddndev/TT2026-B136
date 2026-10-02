//! Exact SQL capability and authority boundary for internal password recovery.
mod catalog;
mod columns;
mod constraints;
mod function_specs;
mod functions;
mod indexes;
mod permissions;
mod triggers;

use application::ApplicationError;
pub(crate) use catalog::validate_schema;
pub(crate) use permissions::{grant_runtime, validate_runtime};
use postgres::GenericClient;
pub(crate) use triggers::validate_audit_triggers;

pub(crate) const MIGRATIONS: &[&str] = &[
    include_str!("../../../../migrations/0028_password_reset_tables.sql"),
    include_str!("../../../../migrations/0028_password_reset_operations.sql"),
    include_str!("../../../../migrations/0028_password_reset_consume.sql"),
    include_str!("../../../../migrations/0028_password_reset_guards.sql"),
    include_str!("../../../../migrations/0028_password_reset_inventory.sql"),
];
const TABLE: &str = "password_reset_capabilities";

pub(crate) fn guard_body(schema: &str) -> Option<String> {
    functions::expected_body("guard_member_access", schema)
}

pub(crate) fn validate_inventory<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let valid: bool = client
        .query_one("SELECT password_reset_inventory_valid()", &[])
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("password reset schema is incomplete or altered".into())
}

fn port(_: postgres::Error) -> ApplicationError {
    ApplicationError::Port("password reset schema operation failed".into())
}
