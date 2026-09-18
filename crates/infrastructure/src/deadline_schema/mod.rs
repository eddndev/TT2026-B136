//! Startup verification of immutable deadline captures and exact dependencies.
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
const TABLES: [&str; 2] = ["case_deadlines", "case_deadline_revisions"];
const HELPERS: [&str; 3] = [
    "deadline_submission(bytea)",
    "deadline_attention_valid(jsonb)",
    "deadline_input_selection(bytea)",
];
const TRIGGERS: [&str; 2] = ["preserve_deadline_history()", "enforce_deadline_sequence()"];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("deadline schema is incomplete or altered; run database migrate with an administrative role".into())
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("deadline schema: {error}"))
}
