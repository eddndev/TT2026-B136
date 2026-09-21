// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../member_atomicity.rs"]
mod member_atomicity;
#[path = "../member_backend.rs"]
mod member_backend;
#[path = "../member_guards.rs"]
mod member_guards;
#[path = "../member_redis.rs"]
mod member_redis;
#[path = "../member_restore.rs"]
mod member_restore;
#[path = "../member_schema.rs"]
mod member_schema;
#[allow(dead_code)]
#[path = "../member_support/mod.rs"]
mod member_support;
