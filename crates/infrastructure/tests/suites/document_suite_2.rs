// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../document_version_edges.rs"]
mod document_version_edges;
#[path = "../document_version_insert_concurrency.rs"]
mod document_version_insert_concurrency;
#[path = "../document_version_migration.rs"]
mod document_version_migration;
#[path = "../document_version_schema.rs"]
mod document_version_schema;
#[allow(dead_code)]
#[path = "../version_database_support/mod.rs"]
mod version_database_support;
