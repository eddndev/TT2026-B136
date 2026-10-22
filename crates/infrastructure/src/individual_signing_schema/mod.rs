//! Exact receipt schema and append-only authority for individual document signatures.
mod checks;
mod columns;
mod constraints;
mod functions;
mod indexes;
mod inventory;
mod specifications;
mod triggers;

use crate::postgres::CatalogClient;
use crate::signing_binding_schema::{catalog, permissions};
use application::ApplicationError;
pub(crate) use inventory::validate_inventory;
use postgres::GenericClient;

pub(crate) const MIGRATIONS: &[&str] = &[include_str!(
    "../../../../migrations/0045_individual_document_signatures.sql"
)];
const TABLES: [&str; 1] = ["individual_document_signatures"];

pub(crate) fn validate_schema<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    catalog::validate_tables(client, &TABLES, incomplete, port)?;
    columns::validate(client)?;
    constraints::validate(client)?;
    indexes::validate(client)?;
    functions::validate(client)?;
    triggers::validate(client)
}

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

pub(crate) fn validate_runtime<C: CatalogClient>(
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
        "individual signing schema is incomplete or altered".into(),
    )
}

fn port(_: postgres::Error) -> ApplicationError {
    ApplicationError::Port("individual signing schema operation failed".into())
}
