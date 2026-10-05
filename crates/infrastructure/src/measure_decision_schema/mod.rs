//! Strict catalog and append-only privileges for complete judicial and administrative measure owners.
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
pub(crate) const MIGRATIONS: &[&str] = &[
    include_str!("../../../../migrations/0034_measure_decisions.sql"),
    include_str!("../../../../migrations/0034_measure_decisions_guards.sql"),
    include_str!("../../../../migrations/0034_measure_decisions_sources.sql"),
    include_str!("../../../../migrations/0034_measure_decisions_complete.sql"),
    include_str!("../../../../migrations/0035_measure_decision_history.sql"),
    include_str!("../../../../migrations/0035_measure_decision_guards.sql"),
    include_str!("../../../../migrations/0035_measure_decision_sources.sql"),
    include_str!("../../../../migrations/0035_measure_decision_complete.sql"),
    include_str!("../../../../migrations/0037_measure_decision_initial_anchor.sql"),
    include_str!("../../../../migrations/0037_measure_decision_anchor_guard.sql"),
    include_str!("../../../../migrations/0038_measure_decision_precautionary_anchor.sql"),
    include_str!("../../../../migrations/0038_measure_decision_capture_guard.sql"),
    include_str!("../../../../migrations/0038_measure_decision_hearing_anchor.sql"),
    include_str!("../../../../migrations/0039_measure_administrative_records.sql"),
    include_str!("../../../../migrations/0039_measure_administrative_capture.sql"),
    include_str!("../../../../migrations/0039_measure_administrative_sources.sql"),
    include_str!("../../../../migrations/0039_measure_administrative_complete.sql"),
    include_str!("../../../../migrations/0039_measure_administrative_guards.sql"),
    include_str!("../../../../migrations/0040_measure_record_families.sql"),
    include_str!("../../../../migrations/0040_measure_decision_capture.sql"),
    include_str!("../../../../migrations/0040_measure_record_sources.sql"),
    include_str!("../../../../migrations/0040_measure_record_complete.sql"),
    include_str!("../../../../migrations/0040_measure_administration_capture.sql"),
    include_str!("../../../../migrations/0040_measure_decision_hearing_records.sql"),
    include_str!("../../../../migrations/0041_measure_administrative_replacement.sql"),
    include_str!("../../../../migrations/0041_measure_administration_capture.sql"),
    include_str!("../../../../migrations/0041_measure_record_sources.sql"),
    include_str!("../../../../migrations/0041_measure_record_complete.sql"),
    include_str!("../../../../migrations/0041_measure_decision_capture.sql"),
    include_str!("../../../../migrations/0041_measure_decision_hearing_records.sql"),
];
const TABLES: [&str; 5] = [
    "case_measure_operations",
    "case_measure_decisions",
    "case_measures",
    "case_measure_revisions",
    "case_measure_administrations",
];
fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "measure decision schema is incomplete or altered".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("measure decision schema: {error}"))
}
