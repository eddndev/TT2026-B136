mod case_administration_support;

use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

#[test]
fn startup_rejects_altered_calendar_canonical_functions_and_columns() {
    for alteration in [
        "ALTER FUNCTION judicial_calendar_values(bytea) SECURITY DEFINER",
        "ALTER FUNCTION judicial_calendar_values(bytea) VOLATILE",
        "CREATE OR REPLACE FUNCTION judicial_calendar_url_valid(value text) RETURNS boolean LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$ BEGIN RETURN true; END $$",
        "ALTER FUNCTION judicial_calendar_values(bytea) RESET search_path",
        "ALTER TABLE judicial_calendar_revisions ENABLE ROW LEVEL SECURITY",
        "ALTER FUNCTION judicial_calendar_values(bytea) STRICT",
        "CREATE OR REPLACE FUNCTION enforce_judicial_calendar_sequence() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END $$",
        "ALTER FUNCTION judicial_calendar_submission(bytea) SECURITY DEFINER",
        "ALTER TABLE judicial_calendar_revisions ALTER COLUMN values_view DROP EXPRESSION",
        "ALTER TABLE judicial_calendar_revisions ALTER COLUMN operation_id DROP NOT NULL",
        "ALTER TABLE judicial_calendar_revisions ADD COLUMN unrecognized text",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.store();
        db.admin.batch_execute(alteration).unwrap();
        assert!(
            PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
            "startup must reject {alteration}"
        );
    }
}

#[test]
fn startup_rejects_direct_inherited_and_column_calendar_mutation_privileges() {
    for privilege in [
        "UPDATE",
        "DELETE",
        "TRUNCATE",
        "TRIGGER",
        "UPDATE(values_digest)",
    ] {
        for inheritance in [0, 1, 2] {
            let inherited = inheritance != 0;
            let Some(mut db) = Fixture::new() else { return };
            let role = if inherited {
                let role = format!("calendar_parent_{}", uuid::Uuid::new_v4().simple());
                db.control
                    .batch_execute(&format!(
                        "CREATE ROLE {role} NOLOGIN; GRANT {role} TO {}; ALTER ROLE {} {}",
                        db.role,
                        db.role,
                        if inheritance == 2 {
                            "NOINHERIT"
                        } else {
                            "INHERIT"
                        }
                    ))
                    .unwrap();
                role
            } else {
                db.role.clone()
            };
            db.admin
                .batch_execute(&format!(
                    "GRANT {privilege} ON judicial_calendar_revisions TO {role}"
                ))
                .unwrap();
            let rejected =
                PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err();
            if inherited {
                db.admin
                    .batch_execute(&format!(
                        "REVOKE {privilege} ON judicial_calendar_revisions FROM {role}"
                    ))
                    .unwrap();
                db.control
                    .batch_execute(&format!("REVOKE {role} FROM {}; DROP ROLE {role}", db.role))
                    .unwrap();
            }
            assert!(rejected, "{privilege}; inherited={inherited}");
        }
    }
}

#[test]
fn startup_requires_runtime_execution_of_calendar_canonical_helpers() {
    for function in [
        "judicial_calendar_url_valid(text)",
        "judicial_calendar_date(bytea,integer)",
        "judicial_calendar_source(bytea,integer)",
        "judicial_calendar_rule(bytea,integer)",
        "judicial_calendar_values(bytea)",
        "judicial_calendar_submission(bytea)",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin
            .batch_execute(&format!(
                "REVOKE EXECUTE ON FUNCTION {function} FROM {}",
                db.role
            ))
            .unwrap();
        assert!(
            PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
            "{function}"
        );
    }
}

#[test]
fn startup_requires_calendar_foreign_keys_unique_operations_and_active_guards() {
    for alteration in [
        "ALTER TABLE judicial_calendar_revisions DROP CONSTRAINT judicial_calendar_operation_unique",
        "ALTER TABLE judicial_calendars DROP CONSTRAINT judicial_calendar_first_revision",
        "ALTER TABLE judicial_calendars ALTER CONSTRAINT judicial_calendar_first_revision NOT DEFERRABLE",
        "ALTER TABLE judicial_calendar_revisions DROP CONSTRAINT judicial_calendar_values_hash;
         ALTER TABLE judicial_calendar_revisions ADD CONSTRAINT judicial_calendar_values_hash
         CHECK(values_digest=sha256(values_canonical)) NOT VALID",
        "DROP TRIGGER judicial_calendar_sequence ON judicial_calendar_revisions",

        "ALTER TABLE judicial_calendar_revisions DISABLE TRIGGER judicial_calendar_immutable",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin.batch_execute(alteration).unwrap();
        assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher))
            .is_err(), "{alteration}");
    }
}

#[test]
fn startup_rejects_calendar_guard_execution_and_protected_object_ownership() {
    for alteration in [
        "GRANT EXECUTE ON FUNCTION enforce_judicial_calendar_sequence() TO",
        "GRANT EXECUTE ON FUNCTION preserve_judicial_calendar_history() TO",
        "ALTER FUNCTION judicial_calendar_values(bytea) OWNER TO",
        "ALTER TABLE judicial_calendars OWNER TO",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin
            .batch_execute(&format!("{alteration} {}", db.role))
            .unwrap();
        assert!(
            PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
            "{alteration}"
        );
    }
}

#[test]
fn startup_rejects_same_named_weakened_checks_and_generated_expressions() {
    for alteration in [
        "ALTER TABLE judicial_calendar_revisions DROP CONSTRAINT judicial_calendar_values_hash; ALTER TABLE judicial_calendar_revisions ADD CONSTRAINT judicial_calendar_values_hash CHECK(true)",
        "ALTER TABLE judicial_calendars DROP CONSTRAINT judicial_calendar_initial_revision; ALTER TABLE judicial_calendars ADD CONSTRAINT judicial_calendar_initial_revision CHECK(initial_revision>0)",
        "ALTER TABLE judicial_calendar_revisions DROP COLUMN status; ALTER TABLE judicial_calendar_revisions ADD COLUMN status text GENERATED ALWAYS AS ('published'::text) STORED",
        "ALTER TABLE judicial_calendar_revisions DROP COLUMN values_view; ALTER TABLE judicial_calendar_revisions ADD COLUMN values_view jsonb GENERATED ALWAYS AS (judicial_calendar_values(values_canonical)||'{\"extra\":true}'::jsonb) STORED",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin.batch_execute(alteration).unwrap();
        assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(), "{alteration}");
    }
}
