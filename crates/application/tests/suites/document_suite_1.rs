// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod crypto;
#[path = "../document_format_batch.rs"]
mod document_format_batch;
#[path = "../document_metadata_query.rs"]
mod document_metadata_query;
#[path = "../document_query.rs"]
mod document_query;
#[path = "../document_record.rs"]
mod document_record;
#[path = "../document_repository_port.rs"]
mod document_repository_port;
#[path = "../document_version_crypto.rs"]
mod document_version_crypto;
#[path = "../document_versions.rs"]
mod document_versions;
#[path = "../document_workflow.rs"]
mod document_workflow;
#[allow(dead_code)]
#[path = "../support/document_workflow.rs"]
mod support;
#[allow(dead_code)]
#[path = "../verification_mocks/mod.rs"]
mod verification_mocks;
