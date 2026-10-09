//! Original upload attribution, separate from legacy audit labels and later versions.
mod catalog;
mod checks;
mod columns;
mod constraints;
mod functions;
mod indexes;
mod inventory;
mod permissions;
mod specifications;
mod triggers;

use application::ApplicationError;
pub(crate) use catalog::validate;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};

pub(crate) const MIGRATIONS: &[&str] = &[include_str!(
    "../../../../migrations/0043_document_upload_origins.sql"
)];
const TABLES: [&str; 1] = ["document_upload_origins"];

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "document upload origin schema is incomplete or altered".into(),
    )
}

fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("document upload origin schema: {error}"))
}
