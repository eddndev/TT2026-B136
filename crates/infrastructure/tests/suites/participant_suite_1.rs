// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../participant_authorization.rs"]
mod participant_authorization;
#[path = "../participant_concurrency.rs"]
mod participant_concurrency;
#[allow(dead_code)]
#[path = "../participant_database_support/mod.rs"]
mod participant_database_support;
#[path = "../participant_failures.rs"]
mod participant_failures;
#[path = "../participant_migration.rs"]
mod participant_migration;
#[path = "../participant_mixed_revisions.rs"]
mod participant_mixed_revisions;
#[path = "../participant_queries.rs"]
mod participant_queries;
#[path = "../participant_restore.rs"]
mod participant_restore;
#[path = "../participant_schema.rs"]
mod participant_schema;
#[path = "../participant_sql_invariants.rs"]
mod participant_sql_invariants;
#[path = "../participant_workflow.rs"]
mod participant_workflow;
#[allow(dead_code)]
#[path = "../typed_participant_database_support/mod.rs"]
mod typed_participant_database_support;
