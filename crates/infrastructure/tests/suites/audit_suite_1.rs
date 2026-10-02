// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../audit_log.rs"]
mod audit_log;
#[path = "../audit_query_postgres.rs"]
mod audit_query_postgres;
#[path = "../audit_query_restore.rs"]
mod audit_query_restore;
#[path = "../audit_query_schema.rs"]
mod audit_query_schema;
#[path = "../audit_query_support/mod.rs"]
mod audit_query_support;
#[path = "../audit_transaction_isolation.rs"]
mod audit_transaction_isolation;
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
