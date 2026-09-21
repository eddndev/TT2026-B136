// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../verification_mocks/mod.rs"]
mod verification_mocks;
#[path = "../verify_document.rs"]
mod verify_document;
#[path = "../verify_document_failures.rs"]
mod verify_document_failures;
