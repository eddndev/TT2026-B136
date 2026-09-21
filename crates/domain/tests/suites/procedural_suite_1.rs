// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../procedural_fact_canonical.rs"]
mod procedural_fact_canonical;
#[path = "../procedural_fact_evidence.rs"]
mod procedural_fact_evidence;
#[path = "../procedural_fact_permissions.rs"]
mod procedural_fact_permissions;
#[path = "../procedural_fact_primitives.rs"]
mod procedural_fact_primitives;
#[allow(dead_code)]
#[path = "../procedural_fact_support/mod.rs"]
mod procedural_fact_support;
#[path = "../procedural_fact_values.rs"]
mod procedural_fact_values;
#[path = "../procedural_resource_canonical.rs"]
mod procedural_resource_canonical;
#[path = "../procedural_resource_commitments.rs"]
mod procedural_resource_commitments;
#[path = "../procedural_resource_identity.rs"]
mod procedural_resource_identity;
#[allow(dead_code)]
#[path = "../procedural_resource_support/mod.rs"]
mod procedural_resource_support;
#[path = "../procedural_resource_values.rs"]
mod procedural_resource_values;
#[path = "../procedural_time_boundaries.rs"]
mod procedural_time_boundaries;
#[path = "../procedural_time_precision.rs"]
mod procedural_time_precision;
