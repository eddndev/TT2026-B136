// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "../deadline_observation_support/mod.rs"]
mod deadline_observation_support;
#[path = "../deadline_service_retained_tracking.rs"]
mod deadline_service_retained_tracking;
#[allow(dead_code)]
#[path = "../deadline_service_support/mod.rs"]
mod deadline_service_support;
#[path = "../deadline_service_tracking_administration.rs"]
mod deadline_service_tracking_administration;
#[path = "../deadline_state_bytes.rs"]
mod deadline_state_bytes;
#[allow(dead_code)]
#[path = "../deadline_support/mod.rs"]
mod deadline_support;
#[path = "../deadline_technical_calendar.rs"]
mod deadline_technical_calendar;
#[path = "../deadline_technical_contract.rs"]
mod deadline_technical_contract;
#[path = "../deadline_technical_early.rs"]
mod deadline_technical_early;
#[path = "../deadline_technical_nochange_administration.rs"]
mod deadline_technical_nochange_administration;
#[path = "../deadline_technical_preparation.rs"]
mod deadline_technical_preparation;
#[allow(dead_code)]
#[path = "../deadline_technical_support/mod.rs"]
mod deadline_technical_support;
#[path = "../deadline_technical_validation.rs"]
mod deadline_technical_validation;
#[path = "../deadline_tracked_administration.rs"]
mod deadline_tracked_administration;
#[path = "../deadline_tracked_calendar_successors.rs"]
mod deadline_tracked_calendar_successors;
#[path = "../deadline_tracked_coalesced_successors.rs"]
mod deadline_tracked_coalesced_successors;
#[allow(dead_code)]
#[path = "../deadline_tracked_support/mod.rs"]
mod deadline_tracked_support;
