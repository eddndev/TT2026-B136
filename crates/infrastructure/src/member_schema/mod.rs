//! Exact catalog and authority boundary for account access and memberships.
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
    "../../../../migrations/0025_member_lifecycle.sql"
)];
const TABLES: [&str; 2] = ["users", "case_memberships"];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("member schema is incomplete or altered".into())
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("member schema: {error}"))
}
