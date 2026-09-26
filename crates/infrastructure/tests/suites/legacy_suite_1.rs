// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../legacy_case_administration.rs"]
mod legacy_case_administration;
#[path = "../legacy_database_import.rs"]
mod legacy_database_import;
#[allow(dead_code)]
#[path = "../legacy_database_support/mod.rs"]
mod legacy_database_support;
#[path = "../legacy_document_metadata.rs"]
mod legacy_document_metadata;
#[path = "../legacy_document_versions.rs"]
mod legacy_document_versions;
#[path = "../legacy_fences.rs"]
mod legacy_fences;
#[path = "../legacy_import.rs"]
mod legacy_import;
#[path = "../legacy_participants.rs"]
mod legacy_participants;
