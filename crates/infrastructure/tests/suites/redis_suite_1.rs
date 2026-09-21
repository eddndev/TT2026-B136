// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../redis_session_atomicity.rs"]
mod redis_session_atomicity;
#[path = "../redis_timeouts.rs"]
mod redis_timeouts;
