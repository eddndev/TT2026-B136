//! Startup verification for durable deadline dispatch positions and jobs.
mod catalog;
mod constraints;
mod functions;
mod indexes;
mod inventory;
mod migrations;
mod permissions;

use application::ApplicationError;

pub(crate) use catalog::validate;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use migrations::SQL as MIGRATIONS;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};

const CURSOR: &str = "deadline_dispatch_cursor";
const JOBS: &str = "deadline_reevaluation_jobs";
const TABLES: [&str; 2] = [CURSOR, JOBS];
const CANDIDATES: &str = "deadline_dispatch_candidates(bigint,uuid,boolean,uuid,boolean,integer)";
const TRIGGER_FUNCTIONS: [&str; 5] = [
    "lock_deadline_dispatch()",
    "preserve_deadline_dispatch()",
    "validate_deadline_reevaluation_job()",
    "reserve_deadline_job_operation()",
    "validate_deadline_dispatch_cursor()",
];

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "deadline dispatch schema or inventory is incomplete or altered".into(),
    )
}

fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("deadline dispatch schema: {error}"))
}
