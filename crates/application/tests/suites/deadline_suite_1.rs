// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[path = "../deadline_administration.rs"]
mod deadline_administration;
#[path = "../deadline_captured_observations.rs"]
mod deadline_captured_observations;
#[path = "../deadline_current_overview.rs"]
mod deadline_current_overview;
#[path = "../deadline_current_service.rs"]
mod deadline_current_service;
#[path = "../deadline_currentness.rs"]
mod deadline_currentness;
#[path = "../deadline_currentness_binding.rs"]
mod deadline_currentness_binding;
#[path = "../deadline_currentness_integrity.rs"]
mod deadline_currentness_integrity;
#[path = "../deadline_currentness_review.rs"]
mod deadline_currentness_review;
#[allow(dead_code)]
#[path = "../deadline_currentness_support/mod.rs"]
mod deadline_currentness_support;
#[path = "../deadline_evaluation_record.rs"]
mod deadline_evaluation_record;
#[path = "../deadline_evaluation_record_calendar.rs"]
mod deadline_evaluation_record_calendar;
#[allow(dead_code)]
#[path = "../deadline_evaluation_record_support/mod.rs"]
mod deadline_evaluation_record_support;
#[path = "../deadline_evaluation_record_validation.rs"]
mod deadline_evaluation_record_validation;
#[path = "../deadline_evaluation_record_wire.rs"]
mod deadline_evaluation_record_wire;
#[allow(dead_code)]
#[path = "../deadline_evaluation_support/mod.rs"]
mod deadline_evaluation_support;
#[allow(dead_code)]
#[path = "../deadline_observation_support/mod.rs"]
mod deadline_observation_support;
#[allow(dead_code)]
#[path = "../deadline_service_support/mod.rs"]
mod deadline_service_support;
#[allow(dead_code)]
#[path = "../deadline_support/mod.rs"]
mod deadline_support;
#[allow(dead_code)]
#[path = "../deadline_technical_support/mod.rs"]
mod deadline_technical_support;
