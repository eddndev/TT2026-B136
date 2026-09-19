//! Startup verification for durable deadline completion and failed attempts.
mod catalog;
mod checks;
mod columns;
mod constraints;
mod function_specs;
mod functions;
mod indexes;
mod inventory;
mod migrations;
mod permissions;
mod reserved;
mod triggers;

use application::ApplicationError;

pub(crate) use catalog::validate;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use migrations::SQL as MIGRATIONS;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};
pub(crate) use reserved::validate as validate_reserved_operations;

const RESULTS: &str = "deadline_reevaluation_results";
const ATTEMPTS: &str = "deadline_reevaluation_attempts";
const TABLES: [&str; 2] = [RESULTS, ATTEMPTS];

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "deadline worker schema or inventory is incomplete or altered".into(),
    )
}

fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("deadline worker schema: {error}"))
}
