mod case_administration_support;
mod deadline_schema_support;

use case_administration_support::Fixture;
use deadline_schema_support::open;
use uuid::Uuid;

#[test]
fn startup_rejects_altered_worker_functions_columns_constraints_indexes_and_triggers() {
    for alteration in [
        "ALTER FUNCTION deadline_worker_cause(uuid) STABLE",
        "ALTER FUNCTION deadline_worker_cause(uuid) SECURITY DEFINER",
        "ALTER FUNCTION deadline_worker_cause(uuid) RESET search_path",
        "CREATE OR REPLACE FUNCTION deadline_worker_cause(job uuid)
            RETURNS jsonb LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog
            AS $$ BEGIN RETURN NULL; END; $$",
        "ALTER FUNCTION deadline_worker_calendar_input(bytea,bytea,bigint) VOLATILE",
        "ALTER TABLE deadline_reevaluation_results ALTER COLUMN base_revision DROP NOT NULL",
        "ALTER TABLE deadline_reevaluation_attempts ALTER COLUMN attempt_number SET DEFAULT 1",
        "ALTER TABLE deadline_reevaluation_results DROP CONSTRAINT deadline_worker_result_shape;
            ALTER TABLE deadline_reevaluation_results ADD CONSTRAINT deadline_worker_result_shape CHECK(true)",
        "ALTER TABLE deadline_reevaluation_attempts ALTER CONSTRAINT deadline_worker_attempt_job
            DEFERRABLE INITIALLY DEFERRED",
        "DROP INDEX deadline_worker_attempt_latest;
            CREATE INDEX deadline_worker_attempt_latest ON deadline_reevaluation_attempts(job_id,attempt_number)",
        "DROP INDEX deadline_worker_attempt_latest;
            CREATE INDEX deadline_worker_attempt_latest ON deadline_reevaluation_attempts(job_id,attempt_number DESC NULLS LAST)",
        "ALTER TABLE deadline_reevaluation_results DISABLE TRIGGER deadline_worker_result_immutable",
        "DROP TRIGGER deadline_worker_attempt_insert ON deadline_reevaluation_attempts;
            CREATE TRIGGER deadline_worker_attempt_insert BEFORE INSERT ON deadline_reevaluation_attempts
                FOR EACH ROW WHEN (false) EXECUTE FUNCTION validate_deadline_worker_attempt()",
        "DROP TRIGGER deadline_worker_revision_result ON case_deadline_revisions;
            CREATE CONSTRAINT TRIGGER deadline_worker_revision_result AFTER INSERT ON case_deadline_revisions
                DEFERRABLE INITIALLY IMMEDIATE FOR EACH ROW EXECUTE FUNCTION require_deadline_worker_result()",
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
fn startup_rejects_missing_excess_and_public_worker_privileges() {
    for alteration in [
        "GRANT INSERT ON deadline_reevaluation_results TO ROLE",
        "GRANT UPDATE(result_capture_digest) ON deadline_reevaluation_results TO ROLE",
        "GRANT UPDATE(checked_base_capture_digest) ON deadline_reevaluation_attempts TO ROLE",
        "GRANT EXECUTE ON FUNCTION validate_deadline_worker_result() TO ROLE",
        "REVOKE INSERT(error_code) ON deadline_reevaluation_attempts FROM ROLE",
        "REVOKE EXECUTE ON FUNCTION deadline_worker_cause(uuid) FROM ROLE",
        "GRANT SELECT ON deadline_reevaluation_results TO PUBLIC",
        "GRANT UPDATE(checked_base_capture_digest) ON deadline_reevaluation_attempts TO PUBLIC",
        "GRANT EXECUTE ON FUNCTION deadline_worker_cause(uuid) TO PUBLIC",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        open(&db).unwrap();
        let alteration = alteration.replace("ROLE", &db.role);
        db.admin.batch_execute(&alteration).unwrap();
        assert!(open(&db).is_err(), "startup accepted {alteration}");
    }
}

#[test]
fn startup_rejects_worker_write_authority_through_inherited_or_settable_roles() {
    for inheritance in ["INHERIT", "NOINHERIT"] {
        let Some(mut db) = Fixture::new() else { return };
        open(&db).unwrap();
        let parent = format!("deadline_worker_parent_{}", Uuid::new_v4().simple());
        db.control
            .batch_execute(&format!(
                "CREATE ROLE {parent} NOLOGIN NOSUPERUSER NOCREATEROLE"
            ))
            .unwrap();
        let altered = (|| -> Result<(), postgres::Error> {
            db.control.batch_execute(&format!(
                "ALTER ROLE {} {inheritance}; GRANT {parent} TO {}",
                db.role, db.role
            ))?;
            db.admin.batch_execute(&format!(
                "GRANT UPDATE(result_capture_digest) ON deadline_reevaluation_results TO {parent}"
            ))
        })();
        let rejected = altered.is_ok() && open(&db).is_err();
        // Remove cross-schema role dependencies before any assertion can fail.
        let cleaned = db.control.batch_execute(&format!(
            "REVOKE UPDATE(result_capture_digest) ON {}.deadline_reevaluation_results FROM {parent};
                REVOKE {parent} FROM {}; DROP ROLE {parent}",
            db.schema, db.role
        ));
        altered.unwrap();
        cleaned.unwrap();
        assert!(rejected, "startup accepted authority via {inheritance}");
        open(&db).unwrap();
    }
}
