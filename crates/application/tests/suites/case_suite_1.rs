// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../case_administration_query.rs"]
mod case_administration_query;
#[allow(dead_code)]
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_administration_workflow.rs"]
mod case_administration_workflow;
#[path = "../case_stage_query.rs"]
mod case_stage_query;
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[path = "../case_workflow.rs"]
mod case_workflow;
