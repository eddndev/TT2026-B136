//! Verifies durable source-change events without requiring historical backfill.
mod catalog;
mod constraints;
mod functions;
mod inventory;
mod permissions;
use application::ApplicationError;
pub(crate) use catalog::validate;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};
const TABLE: &str = "deadline_source_events";
const SEQUENCE: &str = "deadline_source_events_sequence";
const FUNCTIONS: [&str; 3] = [
    "preserve_deadline_source_events()",
    "validate_deadline_source_event()",
    "emit_deadline_source_event()",
];
const SOURCES: [&str; 4] = [
    "case_procedural_fact_revisions",
    "case_hearing_result_revisions",
    "judicial_calendar_revisions",
    "deadline_profile_revisions",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "deadline source event schema or inventory is incomplete or altered".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("deadline source events: {error}"))
}
