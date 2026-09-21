// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../hearing_canonical.rs"]
mod hearing_canonical;
#[path = "../hearing_permissions.rs"]
mod hearing_permissions;
#[path = "../hearing_result_canonical.rs"]
mod hearing_result_canonical;
#[path = "../hearing_result_permissions.rs"]
mod hearing_result_permissions;
#[allow(dead_code)]
#[path = "../hearing_result_support/mod.rs"]
mod hearing_result_support;
#[path = "../hearing_result_time.rs"]
mod hearing_result_time;
#[path = "../hearing_result_values.rs"]
mod hearing_result_values;
#[path = "../hearing_time.rs"]
mod hearing_time;
#[path = "../hearing_values.rs"]
mod hearing_values;
