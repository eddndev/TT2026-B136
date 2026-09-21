// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../case_administration_backend.rs"]
mod case_administration_backend;
#[path = "../case_administration_canonical.rs"]
mod case_administration_canonical;
#[path = "../case_administration_concurrency.rs"]
mod case_administration_concurrency;
#[path = "../case_administration_failures.rs"]
mod case_administration_failures;
#[path = "../case_administration_inventory.rs"]
mod case_administration_inventory;
#[path = "../case_administration_migration.rs"]
mod case_administration_migration;
#[path = "../case_administration_queries.rs"]
mod case_administration_queries;
#[path = "../case_administration_restore.rs"]
mod case_administration_restore;
#[path = "../case_administration_schema.rs"]
mod case_administration_schema;
#[path = "../case_administration_sql.rs"]
mod case_administration_sql;
#[allow(dead_code)]
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_backends.rs"]
mod case_backends;
#[path = "../case_stage_backend.rs"]
mod case_stage_backend;
#[allow(dead_code)]
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
