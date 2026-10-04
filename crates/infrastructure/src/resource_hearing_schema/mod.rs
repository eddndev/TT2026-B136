//! Strict catalog and append-only grants for resource-specific hearing captures.
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
    include_str!("../../../../migrations/0030_resource_hearings.sql"),
    include_str!("../../../../migrations/0030_resource_hearings_guards.sql"),
    include_str!("../../../../migrations/0030_resource_hearings_associations.sql"),
    include_str!("../../../../migrations/0030_resource_hearings_association_guards.sql"),
];
const TABLES: [&str; 2] = ["case_resource_hearings", "case_resource_hearing_revisions"];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "resource hearing schema is incomplete or altered".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("resource hearing schema: {error}"))
}
