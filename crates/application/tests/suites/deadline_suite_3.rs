// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "../deadline_evaluation_support/mod.rs"]
mod deadline_evaluation_support;
#[path = "../deadline_profile_catalog.rs"]
mod deadline_profile_catalog;
#[path = "../deadline_profile_catalog_canonical.rs"]
mod deadline_profile_catalog_canonical;
#[path = "../deadline_profile_catalog_failures.rs"]
mod deadline_profile_catalog_failures;
#[path = "../deadline_profile_catalog_queries.rs"]
mod deadline_profile_catalog_queries;
#[allow(dead_code)]
#[path = "../deadline_profile_catalog_support/mod.rs"]
mod deadline_profile_catalog_support;
#[path = "../deadline_profiled_evaluation.rs"]
mod deadline_profiled_evaluation;
#[path = "../deadline_queries.rs"]
mod deadline_queries;
#[path = "../deadline_receipt_vectors.rs"]
mod deadline_receipt_vectors;
#[path = "../deadline_receipts.rs"]
mod deadline_receipts;
#[path = "../deadline_reevaluation_observations.rs"]
mod deadline_reevaluation_observations;
#[path = "../deadline_reevaluation_receipts.rs"]
mod deadline_reevaluation_receipts;
#[path = "../deadline_responsibles.rs"]
mod deadline_responsibles;
#[path = "../deadline_service_identity.rs"]
mod deadline_service_identity;
#[allow(dead_code)]
#[path = "../deadline_service_support/mod.rs"]
mod deadline_service_support;
#[allow(dead_code)]
#[path = "../deadline_support/mod.rs"]
mod deadline_support;
