use crate::case_administration_support;
use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

fn rejects(alteration: &str) {
    let Some(mut db) = Fixture::new() else { return };
    db.store();
    db.admin.batch_execute(alteration).unwrap();
    assert!(
        PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
        "startup must reject {alteration}"
    );
}

#[test]
fn startup_rejects_changed_fact_columns_defaults_and_generated_projections() {
    for alteration in [
        "ALTER TABLE case_procedural_fact_revisions ALTER COLUMN operation_id DROP NOT NULL",
        "ALTER TABLE case_procedural_fact_revisions ALTER COLUMN values_view DROP EXPRESSION",
    ] {
        rejects(alteration);
    }
}

#[test]
fn startup_rejects_weakened_fact_checks_and_foreign_keys() {
    rejects("ALTER TABLE case_procedural_fact_revisions DROP CONSTRAINT procedural_fact_values_hash;
         ALTER TABLE case_procedural_fact_revisions ADD CONSTRAINT procedural_fact_values_hash CHECK(true)");
}

#[test]
fn startup_rejects_fact_function_body_and_execution_metadata_changes() {
    rejects("ALTER FUNCTION procedural_fact_values(text,bytea) SECURITY DEFINER");
}

#[test]
fn startup_requires_all_fact_guards_without_conditions_or_disabled_foreign_keys() {
    rejects("ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER ALL");
}

#[test]
fn startup_preserves_the_operation_constraint_name_used_for_conflict_mapping() {
    rejects(
        "ALTER TABLE case_procedural_fact_revisions RENAME CONSTRAINT
         procedural_fact_operation_unique TO renamed_fact_operation_unique",
    );
}
