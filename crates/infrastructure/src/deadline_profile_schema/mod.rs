//! Startup verification of immutable deadline profile definitions and receipts.
mod catalog;
mod constraints;
mod expressions;
mod functions;
mod inventory;
mod permissions;
use application::ApplicationError;
pub(crate) use catalog::validate;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};
const TABLES: [&str; 2] = ["deadline_profiles", "deadline_profile_revisions"];
const HELPERS: [&str; 2] = [
    "deadline_profile_definition(bytea)",
    "deadline_profile_submission(bytea)",
];
const TRIGGERS: [&str; 2] = [
    "preserve_deadline_profile_history()",
    "enforce_deadline_profile_sequence()",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("deadline profile schema is incomplete or altered; run database migrate with an administrative role".into())
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("deadline profile schema: {error}"))
}
