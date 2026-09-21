// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../case_stage_canonical_sql.rs"]
mod case_stage_canonical_sql;
#[path = "../case_stage_concurrency.rs"]
mod case_stage_concurrency;
#[path = "../case_stage_constraints.rs"]
mod case_stage_constraints;
#[allow(dead_code)]
#[path = "../case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[path = "../case_stage_failures.rs"]
mod case_stage_failures;
#[path = "../case_stage_guards.rs"]
mod case_stage_guards;
#[path = "../case_stage_legacy.rs"]
mod case_stage_legacy;
#[path = "../case_stage_permissions.rs"]
mod case_stage_permissions;
#[path = "../case_stage_restore.rs"]
mod case_stage_restore;
#[path = "../case_stage_schema.rs"]
mod case_stage_schema;
#[path = "../case_stage_snapshots.rs"]
mod case_stage_snapshots;
#[allow(dead_code)]
#[path = "../../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "../legacy_database_support/mod.rs"]
mod legacy_database_support;
