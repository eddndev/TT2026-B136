// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[path = "../hearing_canonical.rs"]
mod hearing_canonical;
#[path = "../hearing_failures.rs"]
mod hearing_failures;
#[path = "../hearing_failures_ports.rs"]
mod hearing_failures_ports;
#[path = "../hearing_queries.rs"]
mod hearing_queries;
#[path = "../hearing_result_canonical.rs"]
mod hearing_result_canonical;
#[path = "../hearing_result_failures.rs"]
mod hearing_result_failures;
#[path = "../hearing_result_outcomes.rs"]
mod hearing_result_outcomes;
#[path = "../hearing_result_queries.rs"]
mod hearing_result_queries;
#[path = "../hearing_result_reads.rs"]
mod hearing_result_reads;
#[path = "../hearing_result_receipts.rs"]
mod hearing_result_receipts;
#[allow(dead_code)]
#[path = "../hearing_result_support/mod.rs"]
mod hearing_result_support;
#[path = "../hearing_result_workflow.rs"]
mod hearing_result_workflow;
#[allow(dead_code)]
#[path = "../hearing_support/mod.rs"]
mod hearing_support;
#[path = "../hearing_workflow.rs"]
mod hearing_workflow;

#[path = "../deadline_support/mod.rs"]
mod deadline_support;
#[path = "../hearing_derived_deadline_preparation.rs"]
mod hearing_derived_deadline_preparation;
#[allow(dead_code)]
#[path = "../hearing_derived_deadline_support/mod.rs"]
mod hearing_derived_deadline_support;

#[path = "../hearing_derived_deadline_rejections.rs"]
mod hearing_derived_deadline_rejections;
#[path = "../hearing_derived_deadline_review.rs"]
mod hearing_derived_deadline_review;
