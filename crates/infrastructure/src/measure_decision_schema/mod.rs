//! Strict catalog and append-only privileges for complete judicial measure owners.
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
    include_str!("../../../../migrations/0034_measure_decisions.sql"),
    include_str!("../../../../migrations/0034_measure_decisions_guards.sql"),
    include_str!("../../../../migrations/0034_measure_decisions_sources.sql"),
    include_str!("../../../../migrations/0034_measure_decisions_complete.sql"),
    include_str!("../../../../migrations/0035_measure_decision_history.sql"),
    include_str!("../../../../migrations/0035_measure_decision_guards.sql"),
    include_str!("../../../../migrations/0035_measure_decision_sources.sql"),
    include_str!("../../../../migrations/0035_measure_decision_complete.sql"),
];
const TABLES: [&str; 4] = [
    "case_measure_operations",
    "case_measure_decisions",
    "case_measures",
    "case_measure_revisions",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "measure decision schema is incomplete or altered".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("measure decision schema: {error}"))
}
