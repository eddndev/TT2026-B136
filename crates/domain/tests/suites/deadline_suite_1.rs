// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../deadline_arithmetic_calendar.rs"]
mod deadline_arithmetic_calendar;
#[path = "../deadline_arithmetic_civil.rs"]
mod deadline_arithmetic_civil;
#[path = "../deadline_arithmetic_contract.rs"]
mod deadline_arithmetic_contract;
#[path = "../deadline_arithmetic_final_day.rs"]
mod deadline_arithmetic_final_day;
#[path = "../deadline_arithmetic_hours.rs"]
mod deadline_arithmetic_hours;
#[path = "../deadline_days_candidates.rs"]
mod deadline_days_candidates;
#[path = "../deadline_days_support/mod.rs"]
mod deadline_days_support;
#[path = "../deadline_identity.rs"]
mod deadline_identity;
#[path = "../deadline_input_permission.rs"]
mod deadline_input_permission;
#[path = "../deadline_profile_rules.rs"]
mod deadline_profile_rules;
#[path = "../deadline_trigger_contract.rs"]
mod deadline_trigger_contract;
#[path = "../deadline_trigger_hearings.rs"]
mod deadline_trigger_hearings;
#[path = "../deadline_trigger_integrity.rs"]
mod deadline_trigger_integrity;
#[path = "../deadline_trigger_integrity_support/mod.rs"]
mod deadline_trigger_integrity_support;
#[path = "../hearing_result_support/mod.rs"]
mod hearing_result_support;
#[path = "../procedural_fact_support/mod.rs"]
mod procedural_fact_support;
