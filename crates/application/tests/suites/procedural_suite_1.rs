// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[path = "../procedural_fact_administration.rs"]
mod procedural_fact_administration;
#[path = "../procedural_fact_base.rs"]
mod procedural_fact_base;
#[allow(dead_code)]
#[path = "../procedural_fact_base_support/mod.rs"]
mod procedural_fact_base_support;
#[path = "../procedural_fact_commands.rs"]
mod procedural_fact_commands;
#[path = "../procedural_fact_hearings.rs"]
mod procedural_fact_hearings;
#[allow(dead_code)]
#[path = "../procedural_fact_receipt_vector_support/mod.rs"]
mod procedural_fact_receipt_vector_support;
#[path = "../procedural_fact_receipt_vectors.rs"]
mod procedural_fact_receipt_vectors;
#[path = "../procedural_fact_receipts.rs"]
mod procedural_fact_receipts;
#[allow(dead_code)]
#[path = "../procedural_fact_service_support/mod.rs"]
mod procedural_fact_service_support;
#[path = "../procedural_fact_sources.rs"]
mod procedural_fact_sources;
#[allow(dead_code)]
#[path = "../procedural_fact_sources_support/mod.rs"]
mod procedural_fact_sources_support;
#[path = "../procedural_resource_acts.rs"]
mod procedural_resource_acts;
#[path = "../procedural_resource_queries.rs"]
mod procedural_resource_queries;
#[path = "../procedural_resource_service.rs"]
mod procedural_resource_service;
#[path = "../procedural_resource_support/mod.rs"]
mod procedural_resource_support;
#[allow(dead_code)]
#[path = "../procedural_fact_hearing_support/mod.rs"]
mod support;
