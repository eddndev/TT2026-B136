// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[allow(dead_code)]
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "../typed_participant_authorization.rs"]
mod typed_participant_authorization;
#[path = "../typed_participant_backend.rs"]
mod typed_participant_backend;
#[path = "../typed_participant_candidates.rs"]
mod typed_participant_candidates;
#[path = "../typed_participant_canonical_sql.rs"]
mod typed_participant_canonical_sql;
#[path = "../typed_participant_codec.rs"]
mod typed_participant_codec;
#[path = "../typed_participant_concurrency.rs"]
mod typed_participant_concurrency;
#[path = "../typed_participant_credential_absence.rs"]
mod typed_participant_credential_absence;
#[allow(dead_code)]
#[path = "../typed_participant_database_support/mod.rs"]
mod typed_participant_database_support;
#[path = "../typed_participant_declaration_vectors.rs"]
mod typed_participant_declaration_vectors;
#[path = "../typed_participant_failures.rs"]
mod typed_participant_failures;
#[path = "../typed_participant_guards.rs"]
mod typed_participant_guards;
#[path = "../typed_participant_integrity.rs"]
mod typed_participant_integrity;
#[path = "../typed_participant_inventory.rs"]
mod typed_participant_inventory;
#[allow(dead_code)]
#[path = "../typed_participant_service_support/mod.rs"]
mod typed_participant_service_support;
#[allow(dead_code)]
#[path = "../../../domain/tests/typed_participant_vectors_support/mod.rs"]
mod values;
#[allow(dead_code)]
#[path = "../../../domain/tests/typed_participant_vectors_support/mod.rs"]
mod vector_support;
