// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[path = "../deadline_observation_support/mod.rs"]
mod deadline_observation_support;
#[path = "../deadline_service_support/mod.rs"]
mod deadline_service_support;
#[path = "../deadline_support/mod.rs"]
mod deadline_support;
#[path = "../deadline_technical_support/mod.rs"]
mod deadline_technical_support;
#[path = "../deadline_tracked_dependency_successors.rs"]
mod deadline_tracked_dependency_successors;
#[path = "../deadline_tracked_preparation.rs"]
mod deadline_tracked_preparation;
#[path = "../deadline_tracked_queries.rs"]
mod deadline_tracked_queries;
#[path = "../deadline_tracked_records.rs"]
mod deadline_tracked_records;
#[path = "../deadline_tracked_successors.rs"]
mod deadline_tracked_successors;
#[path = "../deadline_tracked_support/mod.rs"]
mod deadline_tracked_support;
#[path = "../deadline_tracking_policy.rs"]
mod deadline_tracking_policy;
#[path = "../deadline_tracking_review.rs"]
mod deadline_tracking_review;
#[path = "../deadline_tracking_storage_administration.rs"]
mod deadline_tracking_storage_administration;
#[path = "../deadline_tracking_storage_decode.rs"]
mod deadline_tracking_storage_decode;
#[path = "../deadline_tracking_storage_restore.rs"]
mod deadline_tracking_storage_restore;
#[path = "../deadline_tracking_storage_support/mod.rs"]
mod deadline_tracking_storage_support;
#[path = "../deadline_tracking_storage_vectors.rs"]
mod deadline_tracking_storage_vectors;
#[path = "../deadline_worker_evidence.rs"]
mod deadline_worker_evidence;
