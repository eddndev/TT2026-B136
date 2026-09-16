mod case_administration_support;

use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

fn rejected(db: &Fixture) -> bool {
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err()
}

#[test]
fn startup_rejects_direct_and_inherited_fact_mutation_privileges() {
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
                let role = format!("fact_parent_{}", uuid::Uuid::new_v4().simple());
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
                    "GRANT {privilege} ON case_procedural_fact_revisions TO {role}"
                ))
                .unwrap();
            let result = rejected(&db);
            if inherited {
                db.admin
                    .batch_execute(&format!(
                        "REVOKE {privilege} ON case_procedural_fact_revisions FROM {role}"
                    ))
                    .unwrap();
                db.control
                    .batch_execute(&format!("REVOKE {role} FROM {}; DROP ROLE {role}", db.role))
                    .unwrap();
            }
            assert!(result, "{privilege}; inherited={inherited}");
        }
    }
}

#[test]
fn runtime_needs_all_fact_canonical_helpers_and_append_permissions() {
    for privilege in [
        "EXECUTE ON FUNCTION procedural_fact_values(text,bytea)",
        "EXECUTE ON FUNCTION procedural_fact_sources(bytea)",
        "EXECUTE ON FUNCTION procedural_fact_submission(bytea)",
        "EXECUTE ON FUNCTION procedural_fact_atom(bytea,integer,text)",
        "SELECT ON case_procedural_facts",
        "INSERT ON case_procedural_fact_revisions",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin
            .batch_execute(&format!("REVOKE {privilege} FROM {}", db.role))
            .unwrap();
        assert!(rejected(&db), "{privilege}");
    }
}

#[test]
fn runtime_cannot_own_fact_objects_or_execute_mutation_guards() {
    for alteration in [
        "GRANT EXECUTE ON FUNCTION enforce_procedural_fact_root() TO",
        "GRANT EXECUTE ON FUNCTION enforce_procedural_fact_sequence() TO",
        "GRANT EXECUTE ON FUNCTION preserve_procedural_fact_history() TO",
        "ALTER FUNCTION procedural_fact_values(text,bytea) OWNER TO",
        "ALTER TABLE case_procedural_facts OWNER TO",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin
            .batch_execute(&format!("{alteration} {}", db.role))
            .unwrap();
        assert!(rejected(&db), "{alteration}");
    }
}
