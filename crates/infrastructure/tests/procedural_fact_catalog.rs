mod case_administration_support;

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
        "ALTER TABLE case_procedural_facts ADD COLUMN unexpected text",
        "ALTER TABLE case_procedural_facts ALTER COLUMN initial_revision SET DEFAULT 2",
        "ALTER TABLE case_procedural_fact_revisions ALTER COLUMN reason SET DEFAULT 'Invented'",
        "ALTER TABLE case_procedural_fact_revisions ENABLE ROW LEVEL SECURITY",
        "ALTER TABLE case_procedural_fact_revisions DROP COLUMN status;
         ALTER TABLE case_procedural_fact_revisions ADD COLUMN status text GENERATED ALWAYS AS
         (CASE action WHEN 'withdraw' THEN 'recorded' ELSE 'recorded' END) STORED",
    ] {
        rejects(alteration);
    }
}

#[test]
fn startup_rejects_weakened_fact_checks_and_foreign_keys() {
    for alteration in [
        "ALTER TABLE case_procedural_fact_revisions DROP CONSTRAINT procedural_fact_operation_unique",
        "ALTER TABLE case_procedural_facts DROP CONSTRAINT procedural_fact_parent_scope",
        "ALTER TABLE case_procedural_facts ALTER CONSTRAINT procedural_fact_first_revision NOT DEFERRABLE",
        "ALTER TABLE case_procedural_fact_revisions DROP CONSTRAINT procedural_fact_values_hash;
         ALTER TABLE case_procedural_fact_revisions ADD CONSTRAINT procedural_fact_values_hash CHECK(true)",
        "ALTER TABLE case_procedural_fact_revisions DROP CONSTRAINT procedural_fact_sources_hash;
         ALTER TABLE case_procedural_fact_revisions ADD CONSTRAINT procedural_fact_sources_hash
         CHECK(sources_digest=sha256(sources_canonical)) NOT VALID",
        "ALTER TABLE case_procedural_fact_revisions DROP CONSTRAINT procedural_fact_administration_shape;
         ALTER TABLE case_procedural_fact_revisions ADD CONSTRAINT procedural_fact_administration_shape
         CHECK(recorded_administration_revision IS NULL OR recorded_administration_digest IS NOT NULL)",
    ] {
        rejects(alteration);
    }
}

#[test]
fn startup_rejects_fact_function_body_and_execution_metadata_changes() {
    for alteration in [
        "ALTER FUNCTION procedural_fact_values(text,bytea) SECURITY DEFINER",
        "ALTER FUNCTION procedural_fact_values(text,bytea) VOLATILE",
        "ALTER FUNCTION procedural_fact_sources(bytea) STRICT",
        "ALTER FUNCTION procedural_fact_submission(bytea) SET search_path=public",
        "ALTER FUNCTION procedural_fact_atom(bytea,integer,text) PARALLEL SAFE",
        "CREATE OR REPLACE FUNCTION procedural_fact_sources(b bytea) RETURNS jsonb
         LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$ BEGIN RETURN '{}'::jsonb; END; $$",
        "CREATE OR REPLACE FUNCTION preserve_procedural_fact_history() RETURNS trigger
         LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NULL; END; $$",
    ] {
        rejects(alteration);
    }
}

#[test]
fn startup_requires_all_fact_guards_without_conditions_or_disabled_foreign_keys() {
    for alteration in [
        "DROP TRIGGER procedural_fact_sequence ON case_procedural_fact_revisions",
        "ALTER TABLE case_procedural_facts DISABLE TRIGGER procedural_fact_root_sources",
        "ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER ALL",
        "DROP TRIGGER procedural_fact_sequence ON case_procedural_fact_revisions;
         CREATE TRIGGER procedural_fact_sequence BEFORE INSERT ON case_procedural_fact_revisions
         FOR EACH ROW WHEN (NEW.revision > 1) EXECUTE FUNCTION enforce_procedural_fact_sequence()",
        "CREATE TRIGGER unrecognized_fact_guard BEFORE INSERT ON case_procedural_facts
         FOR EACH ROW EXECUTE FUNCTION enforce_procedural_fact_root()",
    ] {
        rejects(alteration);
    }
}

#[test]
fn startup_preserves_the_operation_constraint_name_used_for_conflict_mapping() {
    rejects(
        "ALTER TABLE case_procedural_fact_revisions RENAME CONSTRAINT
         procedural_fact_operation_unique TO renamed_fact_operation_unique",
    );
}
