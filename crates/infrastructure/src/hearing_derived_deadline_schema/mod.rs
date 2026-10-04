//! Strict catalog and append-only grants for compound result and deadline origins.
mod catalog;
mod checks;
mod columns;
mod constraints;
mod functions;
mod history;
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
    include_str!("../../../../migrations/0032_hearing_derived_deadlines.sql"),
    include_str!("../../../../migrations/0032_hearing_derived_deadlines_guards.sql"),
];
const TABLES: [&str; 1] = ["case_hearing_derived_deadline_origins"];

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "hearing-derived deadline origin schema is incomplete or altered".into(),
    )
}

fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("hearing-derived deadline origin schema: {error}"))
}
