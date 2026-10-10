//! Exact catalog and append-only authority for global Individual signing history.
pub(crate) mod catalog;
mod checks;
mod columns;
mod constraints;
mod functions;
mod indexes;
mod inventory;
pub(crate) mod permissions;
mod specifications;
mod triggers;

use application::ApplicationError;
pub(crate) use catalog::validate_schema;
pub(crate) use inventory::validate_inventory;
use postgres::GenericClient;

pub(crate) const MIGRATIONS: &[&str] = &[include_str!(
    "../../../../migrations/0044_document_signing_bindings.sql"
)];
const TABLES: [&str; 2] = ["document_signing_bindings", "document_signing_withdrawals"];

pub(crate) fn grant_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    permissions::grant_runtime(
        client,
        role,
        &TABLES,
        columns::names,
        &functions::TRIGGERS,
        port,
    )
}

pub(crate) fn validate_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    permissions::validate_runtime(
        client,
        role,
        &TABLES,
        columns::names,
        &functions::TRIGGERS,
        port,
    )
}

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "document signing schema is incomplete or altered".into(),
    )
}

fn port(_: postgres::Error) -> ApplicationError {
    ApplicationError::Port("document signing schema operation failed".into())
}
