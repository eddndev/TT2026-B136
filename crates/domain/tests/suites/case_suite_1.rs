// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../case_administration.rs"]
mod case_administration;
#[path = "../case_administration_canonical.rs"]
mod case_administration_canonical;
#[path = "../case_stage_canonical.rs"]
mod case_stage_canonical;
#[path = "../case_stage_permissions.rs"]
mod case_stage_permissions;
#[path = "../case_stage_time.rs"]
mod case_stage_time;
#[path = "../case_stage_values.rs"]
mod case_stage_values;
