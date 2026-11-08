use crate::case_administration_support;
use application::ApplicationError;
use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

#[test]
fn startup_rejects_altered_result_canonical_functions_and_columns() {
    for alteration in [
        "ALTER FUNCTION hearing_result_values(bytea) SECURITY DEFINER",
        "ALTER TABLE case_hearing_result_revisions ADD COLUMN unrecognized text",
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
fn startup_rejects_direct_inherited_and_column_result_mutation_privileges() {
    for (privilege, inherited) in [
        ("UPDATE", false),
        ("UPDATE", true),
        ("UPDATE(values_digest)", false),
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let role = if inherited {
            let role = format!("result_parent_{}", uuid::Uuid::new_v4().simple());
            db.control
                .batch_execute(&format!(
                    "CREATE ROLE {role} NOLOGIN; GRANT {role} TO {}",
                    db.role
                ))
                .unwrap();
            role
        } else {
            db.role.clone()
        };
        db.admin
            .batch_execute(&format!(
                "GRANT {privilege} ON case_hearing_result_revisions TO {role}"
            ))
            .unwrap();
        let rejected =
            PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err();
        if inherited {
            db.admin
                .batch_execute(&format!(
                    "REVOKE {privilege} ON case_hearing_result_revisions FROM {role}"
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
fn startup_requires_runtime_execution_of_result_canonical_helpers() {
    let function = "hearing_result_submission(bytea)";
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
fn startup_requires_result_foreign_keys_unique_operations_and_active_guards() {
    assert_missing_operation_unique_is_rejected();
    let alteration =
        "ALTER TABLE case_hearing_result_revisions DISABLE TRIGGER hearing_result_immutable";
    let Some(mut db) = Fixture::new() else { return };
    db.admin.batch_execute(alteration).unwrap();
    assert!(
        PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
        "{alteration}"
    );
}

#[test]
fn startup_rejects_result_guard_execution_and_protected_object_ownership() {
    for alteration in [
        "GRANT EXECUTE ON FUNCTION enforce_hearing_result_root() TO",
        "ALTER TABLE case_hearing_results OWNER TO",
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

fn assert_missing_operation_unique_is_rejected() {
    let Some(mut db) = Fixture::new() else { return };
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    // Remove the dependent origin reference only to expose the missing parent key.
    db.admin.batch_execute(
        "ALTER TABLE case_hearing_derived_deadline_origins DROP CONSTRAINT hearing_derived_deadline_result_operation_fk;
         ALTER TABLE case_hearing_result_revisions DROP CONSTRAINT hearing_result_operation_unique;",
    ).unwrap();
    assert!(
        matches!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)), Err(ApplicationError::InvalidConfiguration(message)) if message == "hearing result schema is incomplete; run database migrate with an administrative role")
    );
    db.admin.batch_execute(
        "ALTER TABLE case_hearing_result_revisions ADD CONSTRAINT hearing_result_operation_unique UNIQUE(operation_id);",
    ).unwrap();
    assert!(
        matches!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)), Err(ApplicationError::InvalidConfiguration(message))
        if message == "hearing-derived deadline origin schema is incomplete or altered")
    );
    db.admin.batch_execute(
        "ALTER TABLE case_hearing_derived_deadline_origins ADD CONSTRAINT hearing_derived_deadline_result_operation_fk
         FOREIGN KEY(operation_id) REFERENCES case_hearing_result_revisions(operation_id);",
    ).unwrap();
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
}
