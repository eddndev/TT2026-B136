//! Exact catalog, runtime authority and startup inventory of bilateral
//! contracts: docs/adr/0086-bilateral-contract-signing.md.
mod checks;
mod columns;
mod constraints;
mod functions;
mod indexes;
mod inventory;
mod permissions;
mod specifications;
mod triggers;

use crate::postgres::CatalogClient;
use crate::signing_binding_schema::catalog;
use application::ApplicationError;
pub(crate) use permissions::{grant_runtime, validate_runtime};
use postgres::GenericClient;

pub(crate) const MIGRATIONS: &[&str] = &[
    include_str!("../../../../migrations/0052_contracts.sql"),
    include_str!("../../../../migrations/0052_contract_events.sql"),
    include_str!("../../../../migrations/0052_contract_signatures.sql"),
];
const TABLES: [&str; 6] = [
    "contracts",
    "contract_signature_requests",
    "contract_events",
    "contract_party_signatures",
    "contract_completions",
    "contract_notifications",
];
/// Every table except the notification inbox, whose delivery columns change.
const APPEND_ONLY: [&str; 5] = [TABLES[0], TABLES[1], TABLES[2], TABLES[3], TABLES[4]];

pub(crate) fn validate_schema<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    catalog::validate_tables(client, &TABLES, incomplete, port)?;
    columns::validate(client)?;
    constraints::validate(client)?;
    indexes::validate(client)?;
    functions::validate(client)?;
    triggers::validate(client)
}

/// Checks stored links and rebuilt statements in SQL.
pub(crate) fn validate_inventory<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    inventory::validate(client)
}

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("contract schema is incomplete or altered".into())
}

fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "contract history is inconsistent; restore a consistent database".into(),
    )
}

fn port(_: postgres::Error) -> ApplicationError {
    ApplicationError::Port("contract schema operation failed".into())
}
