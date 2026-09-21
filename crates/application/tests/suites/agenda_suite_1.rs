// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../agenda_page.rs"]
mod agenda_page;
#[path = "../agenda_projection.rs"]
mod agenda_projection;
#[path = "../agenda_query.rs"]
mod agenda_query;
#[path = "../agenda_service.rs"]
mod agenda_service;
#[allow(dead_code)]
#[path = "../agenda_support/mod.rs"]
mod agenda_support;
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "../deadline_currentness_support/mod.rs"]
mod deadline_currentness_support;
#[allow(dead_code)]
#[path = "../deadline_observation_support/mod.rs"]
mod deadline_observation_support;
#[allow(dead_code)]
#[path = "../deadline_support/mod.rs"]
mod deadline_support;
#[allow(dead_code)]
#[path = "../deadline_technical_support/mod.rs"]
mod deadline_technical_support;
