mod case_administration_support;

use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

#[test]
fn startup_rejects_altered_result_canonical_functions_and_columns() {
    for alteration in [
        "ALTER FUNCTION hearing_result_values(bytea) SECURITY DEFINER",
        "ALTER FUNCTION hearing_result_values(bytea) VOLATILE",
        "ALTER FUNCTION hearing_result_submission(bytea) SECURITY DEFINER",
        "ALTER TABLE case_hearing_result_revisions ALTER COLUMN values_view DROP EXPRESSION",
        "ALTER TABLE case_hearing_result_revisions ALTER COLUMN operation_id DROP NOT NULL",
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
    for privilege in [
        "UPDATE",
        "DELETE",
        "TRUNCATE",
        "TRIGGER",
        "UPDATE(values_digest)",
    ] {
        for inherited in [false, true] {
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
}

#[test]
fn startup_requires_runtime_execution_of_result_canonical_helpers() {
    for function in [
        "hearing_result_values(bytea)",
        "hearing_result_submission(bytea)",
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
fn startup_requires_result_foreign_keys_unique_operations_and_active_guards() {
    for alteration in [
        "ALTER TABLE case_hearing_result_revisions DROP CONSTRAINT hearing_result_operation_unique",
        "ALTER TABLE case_hearing_results DROP CONSTRAINT hearing_result_first_revision",
        "ALTER TABLE case_hearing_results DROP CONSTRAINT hearing_result_anchor",
        "ALTER TABLE case_hearing_results DROP CONSTRAINT hearing_result_continuation",
        "ALTER TABLE case_hearing_results ALTER CONSTRAINT hearing_result_first_revision NOT DEFERRABLE",
        "ALTER TABLE case_hearing_result_revisions DROP CONSTRAINT hearing_result_values_hash;
         ALTER TABLE case_hearing_result_revisions ADD CONSTRAINT hearing_result_values_hash
         CHECK(values_digest=sha256(values_canonical)) NOT VALID",
        "DROP TRIGGER hearing_result_sequence ON case_hearing_result_revisions",
        "ALTER TABLE case_hearing_results DISABLE TRIGGER hearing_result_root_sources",
        "ALTER TABLE case_hearing_result_revisions DISABLE TRIGGER hearing_result_immutable",
        "ALTER FUNCTION enforce_hearing_result_root() SECURITY DEFINER",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin.batch_execute(alteration).unwrap();
        assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher))
            .is_err(), "{alteration}");
    }
}

#[test]
fn startup_rejects_result_guard_execution_and_protected_object_ownership() {
    for alteration in [
        "GRANT EXECUTE ON FUNCTION enforce_hearing_result_root() TO",
        "GRANT EXECUTE ON FUNCTION enforce_hearing_result_sequence() TO",
        "GRANT EXECUTE ON FUNCTION preserve_hearing_result_history() TO",
        "ALTER FUNCTION hearing_result_values(bytea) OWNER TO",
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
