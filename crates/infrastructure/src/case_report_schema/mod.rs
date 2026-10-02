//! Exact schema and authority boundary for durable private case reports.
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
pub(crate) const MIGRATIONS: &[&str] =
    &[include_str!("../../../../migrations/0026_case_reports.sql")];
const TABLES: [&str; 4] = [
    "case_report_jobs",
    "case_report_snapshots",
    "case_report_artifacts",
    "case_report_notices",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration("case report schema is incomplete or altered".into())
}
fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("case report schema", error)
}
