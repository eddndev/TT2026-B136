// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../alert_preferences.rs"]
mod alert_preferences;
#[path = "../alert_query.rs"]
mod alert_query;
#[path = "../alert_service.rs"]
mod alert_service;
#[allow(dead_code)]
#[path = "../alert_support/mod.rs"]
mod alert_support;
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
