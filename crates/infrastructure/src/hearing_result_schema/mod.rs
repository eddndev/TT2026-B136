//! Startup checks for immutable declared sessions and restricted runtime access.

mod catalog;
mod constraints;
mod inventory;
mod permissions;

pub(crate) use catalog::validate;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};

use application::ApplicationError;

const TABLES: [&str; 2] = ["case_hearing_results", "case_hearing_result_revisions"];
const HELPERS: [&str; 3] = [
    "hearing_result_time(bytea,integer)",
    "hearing_result_values(bytea)",
    "hearing_result_submission(bytea)",
];
const TRIGGERS: [&str; 3] = [
    "preserve_hearing_result_history()",
    "enforce_hearing_result_root()",
    "enforce_hearing_result_sequence()",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "hearing result schema is incomplete; run database migrate with an administrative role"
            .into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("hearing result schema: {error}"))
}
