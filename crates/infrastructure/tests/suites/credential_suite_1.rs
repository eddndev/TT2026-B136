// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../credential_trust_backend.rs"]
mod credential_trust_backend;
#[path = "../credential_trust_concurrency.rs"]
mod credential_trust_concurrency;
#[allow(dead_code)]
#[path = "../credential_trust_database_support/mod.rs"]
mod credential_trust_database_support;
#[path = "../credential_trust_permissions.rs"]
mod credential_trust_permissions;
#[path = "../credential_trust_restore.rs"]
mod credential_trust_restore;
#[path = "../credential_trust_schema.rs"]
mod credential_trust_schema;
