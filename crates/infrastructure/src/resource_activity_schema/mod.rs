//! Strict catalog and append-only grants for independent activity associations.
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
    include_str!("../../../../migrations/0023_resource_activities.sql"),
    include_str!("../../../../migrations/0023_resource_activity_guards.sql"),
];
const TABLES: [&str; 2] = [
    "case_resource_activity_associations",
    "case_resource_activity_association_revisions",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "resource activity schema is incomplete or altered".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("resource activity schema: {error}"))
}
