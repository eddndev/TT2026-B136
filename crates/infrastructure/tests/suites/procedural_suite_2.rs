// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "../hearing_database_support/mod.rs"]
mod hearing_database_support;
#[path = "../hearing_result_database_support/mod.rs"]
mod hearing_result_database_support;
#[path = "../hearing_result_revalidation_support/mod.rs"]
mod hearing_result_revalidation_support;
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;
#[path = "../procedural_fact_source_administration.rs"]
mod procedural_fact_source_administration;
#[path = "../procedural_resource_act_root.rs"]
mod procedural_resource_act_root;
#[path = "../procedural_resource_atomicity.rs"]
mod procedural_resource_atomicity;
#[path = "../procedural_resource_backend.rs"]
mod procedural_resource_backend;
#[path = "../procedural_resource_inventory.rs"]
mod procedural_resource_inventory;
#[path = "../procedural_resource_restore.rs"]
mod procedural_resource_restore;
#[path = "../procedural_resource_schema.rs"]
mod procedural_resource_schema;
#[path = "../procedural_resource_support/mod.rs"]
mod procedural_resource_support;
