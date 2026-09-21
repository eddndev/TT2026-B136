//! Strict catalog, immutable history and grants for the Owner integrity inbox.
mod catalog;
mod checks;
mod columns;
mod constraints;
mod functions;
mod indexes;
mod permissions;
mod specifications;
mod triggers;
pub(crate) use crate::document_postgres::validate_integrity_inventory as validate_inventory;
use application::ApplicationError;
pub(crate) use catalog::validate;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};
pub(crate) const MIGRATIONS: &[&str] = &[include_str!(
    "../../../../migrations/0024_document_integrity.sql"
)];
const TABLES: [&str; 1] = ["document_integrity_incidents"];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "document integrity schema is incomplete or altered".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("document integrity schema: {error}"))
}
