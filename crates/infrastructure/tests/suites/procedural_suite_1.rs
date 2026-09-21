// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "../legacy_database_support/mod.rs"]
mod legacy_database_support;
#[path = "../procedural_fact_administration_shape.rs"]
mod procedural_fact_administration_shape;
#[path = "../procedural_fact_atomicity.rs"]
mod procedural_fact_atomicity;
#[path = "../procedural_fact_authorization.rs"]
mod procedural_fact_authorization;
#[path = "../procedural_fact_backend.rs"]
mod procedural_fact_backend;
#[path = "../procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;
#[path = "../procedural_fact_catalog.rs"]
mod procedural_fact_catalog;
#[path = "../procedural_fact_concurrency.rs"]
mod procedural_fact_concurrency;
#[allow(dead_code)]
#[path = "../procedural_fact_concurrency_support/mod.rs"]
mod procedural_fact_concurrency_support;
#[path = "../procedural_fact_import.rs"]
mod procedural_fact_import;
#[path = "../procedural_fact_inventory.rs"]
mod procedural_fact_inventory;
#[path = "../procedural_fact_permissions.rs"]
mod procedural_fact_permissions;
#[path = "../procedural_fact_queries.rs"]
mod procedural_fact_queries;
#[path = "../procedural_fact_restore.rs"]
mod procedural_fact_restore;
#[path = "../procedural_fact_schema.rs"]
mod procedural_fact_schema;
