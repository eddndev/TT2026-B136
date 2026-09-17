//! Startup verification for immutable jurisdictional calendar revisions.
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
const TABLES: [&str; 2] = ["judicial_calendars", "judicial_calendar_revisions"];
const HELPERS: [&str; 6] = [
    "judicial_calendar_url_valid(text)",
    "judicial_calendar_date(bytea,integer)",
    "judicial_calendar_source(bytea,integer)",
    "judicial_calendar_rule(bytea,integer)",
    "judicial_calendar_values(bytea)",
    "judicial_calendar_submission(bytea)",
];
const TRIGGERS: [&str; 2] = [
    "preserve_judicial_calendar_history()",
    "enforce_judicial_calendar_sequence()",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("judicial calendar schema is incomplete or altered; run database migrate with an administrative role".into())
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("judicial calendar schema: {error}"))
}
