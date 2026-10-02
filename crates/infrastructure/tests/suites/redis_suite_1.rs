// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../redis_session_atomicity.rs"]
mod redis_session_atomicity;
#[path = "../redis_session_concurrency.rs"]
mod redis_session_concurrency;
#[path = "../redis_session_lifetime.rs"]
mod redis_session_lifetime;
#[path = "../redis_session_support.rs"]
mod redis_session_support;
#[path = "../redis_session_validation.rs"]
mod redis_session_validation;
#[path = "../redis_timeouts.rs"]
mod redis_timeouts;
