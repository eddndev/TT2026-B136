// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../agenda_page.rs"]
mod agenda_page;
#[path = "../agenda_precautionary_hearings.rs"]
mod agenda_precautionary_hearings;
#[path = "../agenda_projection.rs"]
mod agenda_projection;
#[path = "../agenda_query.rs"]
mod agenda_query;
#[path = "../agenda_resource_hearings.rs"]
mod agenda_resource_hearings;
#[path = "../agenda_service.rs"]
mod agenda_service;
#[path = "../agenda_support/mod.rs"]
mod agenda_support;
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[path = "../deadline_currentness_support/mod.rs"]
mod deadline_currentness_support;
#[path = "../deadline_observation_support/mod.rs"]
mod deadline_observation_support;
#[path = "../deadline_support/mod.rs"]
mod deadline_support;
#[path = "../deadline_technical_support/mod.rs"]
mod deadline_technical_support;
