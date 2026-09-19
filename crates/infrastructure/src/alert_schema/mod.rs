//! Startup verification for durable personal alerts and their runtime grants.
mod catalog;
mod checks;
mod columns;
mod constraints;
mod functions;
mod indexes;
mod permissions;
mod specifications;
mod triggers;

use application::ApplicationError;
pub(crate) use catalog::validate;
pub(crate) use permissions::{grant_runtime, validate_runtime_role};

pub(crate) const MIGRATIONS: &[&str] = &[
    include_str!("../../../../migrations/0021_alert_values.sql"),
    include_str!("../../../../migrations/0021_alert_tables.sql"),
    include_str!("../../../../migrations/0021_alert_guards.sql"),
];
const TABLES: [&str; 8] = [
    "alert_preferences",
    "alert_subject_state",
    "alert_scan_cursor",
    "alert_schedule",
    "alert_notifications",
    "alert_read_receipts",
    "alert_email_outbox",
    "alert_email_attempts",
];
const CURSOR: &str = "alert_scan_cursor";

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("alert schema is incomplete or altered".into())
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("alert schema: {error}"))
}
