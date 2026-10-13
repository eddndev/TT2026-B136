// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../identity_secret.rs"]
mod identity_secret;
#[path = "../identity_transactions.rs"]
mod identity_transactions;

#[path = "../ca_identity_cases/mod.rs"]
mod ca_identity;
