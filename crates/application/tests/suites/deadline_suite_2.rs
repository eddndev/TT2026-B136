// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[path = "../deadline_evidence_integrity.rs"]
mod deadline_evidence_integrity;
#[path = "../deadline_human_command.rs"]
mod deadline_human_command;
#[path = "../deadline_input_service.rs"]
mod deadline_input_service;
#[allow(dead_code)]
#[path = "../deadline_input_service_support/mod.rs"]
mod deadline_input_service_support;
use deadline_support::evaluation::inputs as deadline_input_support;
#[path = "../deadline_legacy_observations.rs"]
mod deadline_legacy_observations;
#[path = "../deadline_legacy_successor_observations.rs"]
mod deadline_legacy_successor_observations;
#[path = "../deadline_legacy_vectors.rs"]
mod deadline_legacy_vectors;
#[path = "../deadline_manual_legacy_observations.rs"]
mod deadline_manual_legacy_observations;
#[path = "../deadline_observation_support/mod.rs"]
mod deadline_observation_support;
#[path = "../deadline_observations.rs"]
mod deadline_observations;
#[path = "../deadline_observations_evidence.rs"]
mod deadline_observations_evidence;
#[path = "../deadline_observations_offsets.rs"]
mod deadline_observations_offsets;
#[path = "../deadline_observations_validation.rs"]
mod deadline_observations_validation;
#[path = "../deadline_preparation.rs"]
mod deadline_preparation;
#[path = "../deadline_support/mod.rs"]
mod deadline_support;
#[path = "../deadline_technical_support/mod.rs"]
mod deadline_technical_support;
#[path = "../deadline_tracked_support/mod.rs"]
mod deadline_tracked_support;
