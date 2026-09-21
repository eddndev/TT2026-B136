// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../typed_participant_values.rs"]
mod typed_participant_values;
#[path = "../typed_participant_vectors.rs"]
mod typed_participant_vectors;
#[allow(dead_code)]
#[path = "../typed_participant_vectors_support/mod.rs"]
mod typed_participant_vectors_support;
