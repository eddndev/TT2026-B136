// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[allow(dead_code)]
#[path = "../document_content_support/mod.rs"]
mod document_content_support;
#[path = "../document_integrity_backend.rs"]
mod document_integrity_backend;
#[path = "../document_integrity_inventory.rs"]
mod document_integrity_inventory;
#[path = "../document_integrity_restore.rs"]
mod document_integrity_restore;
#[path = "../document_integrity_schema.rs"]
mod document_integrity_schema;
#[path = "../document_metadata_authorization.rs"]
mod document_metadata_authorization;
#[path = "../document_metadata_concurrency.rs"]
mod document_metadata_concurrency;
#[path = "../document_metadata_edges.rs"]
mod document_metadata_edges;
#[path = "../document_metadata_failures.rs"]
mod document_metadata_failures;
#[path = "../document_metadata_migration.rs"]
mod document_metadata_migration;
#[path = "../document_metadata_queries.rs"]
mod document_metadata_queries;
#[path = "../document_metadata_restore.rs"]
mod document_metadata_restore;
#[path = "../document_metadata_schema.rs"]
mod document_metadata_schema;
#[allow(dead_code)]
#[path = "../metadata_database_support/mod.rs"]
mod metadata_database_support;
#[allow(dead_code)]
#[path = "../version_database_support/mod.rs"]
mod version_database_support;
