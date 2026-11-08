use crate::{case_administration_support, deadline_schema_support};
use case_administration_support::Fixture;
use deadline_schema_support::open;
use postgres::error::SqlState;
use serde_json::Value;
use uuid::Uuid;

#[test]
fn startup_rejects_changed_dispatch_columns_constraints_indexes_triggers_and_functions() {
    for alteration in [
        "ALTER TABLE deadline_reevaluation_jobs ALTER COLUMN created_at_seconds DROP NOT NULL",
        "ALTER TABLE deadline_dispatch_cursor DISABLE TRIGGER deadline_dispatch_lock",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        open(&db).unwrap();
        db.admin
            .batch_execute(alteration)
            .unwrap_or_else(|error| panic!("cannot apply {alteration}: {error}"));
        assert!(open(&db).is_err(), "startup accepted {alteration}");
    }
}

#[test]
fn runtime_can_update_only_cursor_positions_and_cannot_rewrite_jobs_or_guards() {
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    let before = dispatch_snapshot(&mut db);
    let mut runtime = db.runtime();
    for column in [
        "completed_event_sequence",
        "active_event_sequence",
        "after_deadline_id",
        "bootstrap_after_deadline_id",
    ] {
        let allowed: bool = runtime
            .query_one(
                "SELECT has_column_privilege(current_user,'deadline_dispatch_cursor',$1,'UPDATE')",
                &[&column],
            )
            .unwrap()
            .get(0);
        assert!(allowed, "missing UPDATE({column})");
        assert_eq!(
            runtime
                .execute(
                    &format!("UPDATE deadline_dispatch_cursor SET {column}={column}"),
                    &[],
                )
                .unwrap(),
            1
        );
    }
    for column in [
        "id",
        "operation_id",
        "deadline_id",
        "case_id",
        "event_sequence",
        "bootstrap_policy_version",
        "created_at_seconds",
        "created_at_nanoseconds",
    ] {
        let allowed: bool = runtime
            .query_one(
                "SELECT has_column_privilege(current_user,'deadline_reevaluation_jobs',$1,'INSERT')",
                &[&column],
            )
            .unwrap()
            .get(0);
        assert!(allowed, "missing job INSERT({column})");
    }
    let broad: bool = runtime
        .query_one(
            "SELECT has_table_privilege(current_user,'deadline_dispatch_cursor','INSERT,UPDATE')
            OR has_column_privilege(current_user,'deadline_dispatch_cursor','singleton','UPDATE')
            OR has_any_column_privilege(current_user,'deadline_reevaluation_jobs','UPDATE')",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(!broad);
    let signature = candidate_signature(&mut db);
    let executable: bool = runtime
        .query_one(
            "SELECT has_function_privilege(current_user,$1,'EXECUTE')",
            &[&signature],
        )
        .unwrap()
        .get(0);
    assert!(executable);
    for forbidden in [
        "UPDATE deadline_reevaluation_jobs SET id=id",
        "DELETE FROM deadline_reevaluation_jobs",
        "TRUNCATE deadline_reevaluation_jobs",
        "INSERT INTO deadline_dispatch_cursor(singleton) VALUES(true)",
        "UPDATE deadline_dispatch_cursor SET singleton=singleton",
        "DELETE FROM deadline_dispatch_cursor",
        "TRUNCATE deadline_dispatch_cursor",
        "ALTER TABLE deadline_reevaluation_jobs DISABLE TRIGGER USER",
        "ALTER FUNCTION reserve_deadline_job_operation() SECURITY DEFINER",
        "SELECT lock_deadline_dispatch()",
    ] {
        let error = runtime.batch_execute(forbidden).unwrap_err();
        assert_eq!(
            error.code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE),
            "unexpected rejection for {forbidden}: {error}"
        );
    }
    assert_eq!(dispatch_snapshot(&mut db), before);
    open(&db).unwrap();
}

#[test]
fn startup_rejects_missing_excess_public_column_and_inherited_dispatch_privileges() {
    for alteration in [
        "GRANT UPDATE(singleton) ON deadline_dispatch_cursor TO ROLE",
        "GRANT UPDATE ON deadline_dispatch_cursor TO ROLE",
        "GRANT EXECUTE ON FUNCTION reserve_deadline_job_operation() TO ROLE",
        "REVOKE UPDATE(bootstrap_after_deadline_id) ON deadline_dispatch_cursor FROM ROLE",
        "GRANT SELECT ON deadline_reevaluation_jobs TO PUBLIC",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        open(&db).unwrap();
        let alteration = alteration.replace("ROLE", &db.role);
        db.admin.batch_execute(&alteration).unwrap();
        assert!(open(&db).is_err(), "startup accepted {alteration}");
    }
    let inheritance = "NOINHERIT";
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    let parent = format!("deadline_dispatch_parent_{}", Uuid::new_v4().simple());
    db.control
        .batch_execute(&format!(
            "CREATE ROLE {parent} NOLOGIN NOSUPERUSER NOCREATEROLE"
        ))
        .unwrap();
    let altered = (|| -> Result<(), postgres::Error> {
        db.control.batch_execute(&format!(
            "GRANT {parent} TO {}; ALTER ROLE {} {inheritance}",
            db.role, db.role
        ))?;
        db.admin.batch_execute(&format!(
            "GRANT UPDATE(singleton) ON deadline_dispatch_cursor TO {parent}"
        ))
    })();
    let rejected = altered.is_ok() && open(&db).is_err();
    // Clean up before any assertion, including a rejected grant setup.
    let cleaned = db.control.batch_execute(&format!(
        "REVOKE UPDATE(singleton) ON {}.deadline_dispatch_cursor FROM {parent};
            REVOKE {parent} FROM {}; DROP ROLE {parent}",
        db.schema, db.role
    ));
    altered.unwrap();
    cleaned.unwrap();
    assert!(
        rejected,
        "startup accepted indirect authority via {inheritance}"
    );
    open(&db).unwrap();
}

fn candidate_signature(db: &mut Fixture) -> String {
    db.admin
        .query_one(
            "SELECT p.oid::regprocedure::text FROM pg_proc p
        JOIN pg_class c ON c.oid='deadline_dispatch_cursor'::regclass
        WHERE p.pronamespace=c.relnamespace AND p.proname='deadline_dispatch_candidates'",
            &[],
        )
        .unwrap()
        .get(0)
}

fn dispatch_snapshot(db: &mut Fixture) -> Value {
    db.admin
        .query_one(
            "SELECT jsonb_build_object(
            'cursor',(SELECT to_jsonb(c) FROM deadline_dispatch_cursor c),
            'jobs',(SELECT jsonb_agg(to_jsonb(j) ORDER BY id) FROM deadline_reevaluation_jobs j),
            'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))",
            &[],
        )
        .unwrap()
        .get(0)
}
