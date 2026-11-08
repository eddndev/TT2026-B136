use crate::case_administration_support;
use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

#[test]
fn startup_rejects_altered_calendar_canonical_functions_and_columns() {
    for alteration in [
        "ALTER FUNCTION judicial_calendar_values(bytea) SECURITY DEFINER",
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
    for (privilege, inherited) in [
        ("UPDATE", false),
        ("UPDATE", true),
        ("UPDATE(values_digest)", false),
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let role = if inherited {
            let role = format!("calendar_parent_{}", uuid::Uuid::new_v4().simple());
            db.control
                .batch_execute(&format!(
                    "CREATE ROLE {role} NOLOGIN; GRANT {role} TO {}; ALTER ROLE {} INHERIT",
                    db.role, db.role
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

#[test]
fn startup_requires_runtime_execution_of_calendar_canonical_helpers() {
    let function = "judicial_calendar_submission(bytea)";
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

#[test]
fn startup_requires_calendar_foreign_keys_unique_operations_and_active_guards() {
    let alteration =
        "ALTER TABLE judicial_calendar_revisions DISABLE TRIGGER judicial_calendar_immutable";
    let Some(mut db) = Fixture::new() else { return };
    db.admin.batch_execute(alteration).unwrap();
    assert!(
        PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
        "{alteration}"
    );
}

#[test]
fn startup_rejects_calendar_guard_execution_and_protected_object_ownership() {
    for alteration in [
        "GRANT EXECUTE ON FUNCTION enforce_judicial_calendar_sequence() TO",
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
    let alteration = "ALTER TABLE judicial_calendar_revisions DROP CONSTRAINT judicial_calendar_values_hash; ALTER TABLE judicial_calendar_revisions ADD CONSTRAINT judicial_calendar_values_hash CHECK(true)";
    let Some(mut db) = Fixture::new() else { return };
    db.admin.batch_execute(alteration).unwrap();
    assert!(
        PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
        "{alteration}"
    );
}
