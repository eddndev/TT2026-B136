// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[path = "../member_query.rs"]
mod member_query;
#[path = "../member_service.rs"]
mod member_service;
#[allow(dead_code)]
#[path = "../member_support/mod.rs"]
mod member_support;
