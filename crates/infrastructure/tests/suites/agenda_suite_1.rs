// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../agenda_backend.rs"]
mod agenda_backend;
#[path = "../agenda_backend_support/mod.rs"]
mod agenda_backend_support;
#[path = "../agenda_concurrency.rs"]
mod agenda_concurrency;
#[path = "../agenda_guards.rs"]
mod agenda_guards;
#[path = "../agenda_pagination.rs"]
mod agenda_pagination;
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "../deadline_backend_support/mod.rs"]
mod deadline_backend_support;
#[path = "../deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;

#[path = "../agenda_resource_hearings.rs"]
mod agenda_resource_hearings;
#[path = "../procedural_resource_support/mod.rs"]
mod procedural_resource_support;
#[path = "../resource_activity_support/mod.rs"]
mod resource_activity_support;
#[path = "../resource_hearing_database_support/mod.rs"]
mod resource_hearing_database_support;
