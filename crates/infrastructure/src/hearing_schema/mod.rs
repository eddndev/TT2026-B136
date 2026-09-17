//! Startup checks for immutable hearing history and restricted runtime access.

mod catalog;
mod constraints;
mod inventory;
mod permissions;

pub(crate) use catalog::validate;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};

use application::ApplicationError;

const TABLES: [&str; 2] = ["case_hearings", "case_hearing_revisions"];
const HELPERS: [&str; 3] = [
    "hearing_text(bytea,integer,integer,boolean)",
    "hearing_values(bytea)",
    "hearing_submission(bytea)",
];
const TRIGGERS: [&str; 2] = ["preserve_hearing_history()", "enforce_hearing_sequence()"];

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "hearing schema is incomplete; run database migrate with an administrative role".into(),
    )
}
fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "hearing inventory is inconsistent; restore a consistent database".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("hearing schema: {error}"))
}
