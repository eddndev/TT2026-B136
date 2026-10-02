// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
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
#[path = "../procedural_resource_support/mod.rs"]
mod procedural_resource_support;
#[path = "../resource_activity_access.rs"]
mod resource_activity_access;
#[path = "../resource_activity_atomicity.rs"]
mod resource_activity_atomicity;
#[path = "../resource_activity_backend.rs"]
mod resource_activity_backend;
#[path = "../resource_activity_inventory.rs"]
mod resource_activity_inventory;
#[path = "../resource_activity_read_clock.rs"]
mod resource_activity_read_clock;
#[path = "../resource_activity_restore.rs"]
mod resource_activity_restore;
#[path = "../resource_activity_schema.rs"]
mod resource_activity_schema;
#[path = "../resource_activity_support/mod.rs"]
mod resource_activity_support;
