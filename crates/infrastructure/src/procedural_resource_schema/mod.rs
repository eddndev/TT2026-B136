//! Exact schema, append-only grants and historical receipt inventory.
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
pub(crate) const MIGRATIONS: &[&str] = &[
    include_str!("../../../../migrations/0022_procedural_resources.sql"),
    include_str!("../../../../migrations/0022_procedural_resource_guards.sql"),
];
const TABLES: [&str; 3] = [
    "case_procedural_resources",
    "case_procedural_resource_acts",
    "case_procedural_resource_revisions",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "procedural resource schema is incomplete or altered".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("procedural resource schema: {error}"))
}
