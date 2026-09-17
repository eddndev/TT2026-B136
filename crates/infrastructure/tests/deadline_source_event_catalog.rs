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
fn startup_requires_the_emitter_on_each_source_table_and_immutable_event_guard() {
    for alteration in [
        "DROP TRIGGER deadline_source_emit ON case_procedural_fact_revisions",
        "ALTER TABLE case_hearing_result_revisions DISABLE TRIGGER deadline_source_emit",
        "ALTER TABLE judicial_calendar_revisions DISABLE TRIGGER deadline_source_emit",
        "DROP TRIGGER deadline_source_immutable ON deadline_source_events",
        "ALTER TABLE deadline_source_events DISABLE TRIGGER deadline_source_insert",
        "DROP TRIGGER deadline_source_emit ON case_procedural_fact_revisions;
         CREATE TRIGGER deadline_source_emit AFTER INSERT ON case_procedural_fact_revisions
         FOR EACH ROW WHEN (NEW.revision>1) EXECUTE FUNCTION emit_deadline_source_event()",
    ] {
        rejects(alteration);
    }
}

#[test]
fn startup_rejects_weakened_types_references_generated_discriminators_and_sequence() {
    for alteration in [
        "ALTER TABLE deadline_source_events ALTER COLUMN operation_id DROP NOT NULL",
        "ALTER TABLE deadline_source_events ALTER COLUMN fact_family DROP EXPRESSION",
        "ALTER TABLE deadline_source_events ADD COLUMN unexpected text",
        "ALTER TABLE deadline_source_events ALTER COLUMN sequence SET DEFAULT 1",
        "ALTER TABLE deadline_source_events ENABLE ROW LEVEL SECURITY",
        "ALTER TABLE deadline_source_events DROP CONSTRAINT deadline_source_revision_unique",
        "ALTER TABLE deadline_source_events DROP CONSTRAINT deadline_source_fact_revision",
        "ALTER TABLE deadline_source_events DROP CONSTRAINT deadline_source_hearing_scope",
        "ALTER TABLE deadline_source_events DROP CONSTRAINT deadline_source_calendar_revision",
        "ALTER SEQUENCE deadline_source_events_sequence OWNED BY NONE",
        "ALTER SEQUENCE deadline_source_events_sequence INCREMENT BY 2",
        "ALTER SEQUENCE deadline_source_events_sequence CYCLE",
    ] {
        rejects(alteration);
    }
}

#[test]
fn startup_rejects_altered_event_function_bodies_or_execution_metadata() {
    for alteration in [
        "ALTER FUNCTION emit_deadline_source_event() SECURITY DEFINER",
        "ALTER FUNCTION validate_deadline_source_event() STABLE",
        "ALTER FUNCTION preserve_deadline_source_events() SET search_path=public",
        "CREATE OR REPLACE FUNCTION validate_deadline_source_event() RETURNS trigger
         LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$",
    ] {
        rejects(alteration);
    }
}

#[test]
fn runtime_role_validation_rejects_event_sequence_or_trigger_bypass_privileges() {
    for privilege in [
        "UPDATE ON deadline_source_events",
        "INSERT(sequence) ON deadline_source_events",
        "UPDATE ON SEQUENCE deadline_source_events_sequence",
        "EXECUTE ON FUNCTION emit_deadline_source_event()",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin
            .batch_execute(&format!("GRANT {privilege} TO {}", db.role))
            .unwrap();
        assert!(
            PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
            "startup must reject {privilege}"
        );
    }
}
