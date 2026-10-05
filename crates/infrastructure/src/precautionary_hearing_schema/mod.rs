//! Strict catalog and append-only grants for precautionary hearing captures.
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
    include_str!("../../../../migrations/0033_precautionary_hearings.sql"),
    include_str!("../../../../migrations/0033_precautionary_hearings_guards.sql"),
    include_str!("../../../../migrations/0036_precautionary_hearing_review.sql"),
    include_str!("../../../../migrations/0040_precautionary_hearing_records.sql"),
    include_str!("../../../../migrations/0041_precautionary_hearing_records.sql"),
];
const TABLES: [&str; 2] = [
    "case_precautionary_hearings",
    "case_precautionary_hearing_revisions",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "precautionary hearing schema is incomplete or altered".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("precautionary hearing schema: {error}"))
}
