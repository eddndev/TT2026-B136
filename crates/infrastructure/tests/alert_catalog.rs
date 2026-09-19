mod case_administration_support;
mod deadline_schema_support;

use case_administration_support::Fixture;
use deadline_schema_support::open;
use postgres::error::SqlState;
use uuid::Uuid;

#[test]
fn alert_startup_rejects_altered_tables_columns_constraints_indexes_functions_and_triggers() {
    for alteration in [
        "ALTER TABLE alert_preferences SET UNLOGGED",
        "ALTER TABLE alert_schedule ENABLE ROW LEVEL SECURITY",
        "ALTER TABLE alert_subject_state ADD COLUMN unexpected text",
        "ALTER TABLE alert_notifications ALTER COLUMN payload DROP NOT NULL",
        "ALTER TABLE alert_email_outbox ALTER COLUMN sequence SET DEFAULT 0",
        "ALTER TABLE alert_scan_cursor ALTER COLUMN next_nanos TYPE smallint",
        "ALTER TABLE alert_schedule ALTER COLUMN occurrence_key TYPE text COLLATE \"default\"",
        "ALTER TABLE alert_preferences DROP CONSTRAINT alert_preferences_operation_id_key",
        "ALTER TABLE alert_read_receipts DROP CONSTRAINT alert_read_receipts_recipient_fkey;
            ALTER TABLE alert_read_receipts ADD CONSTRAINT alert_read_receipts_recipient_fkey
                FOREIGN KEY(recipient) REFERENCES users(id) ON DELETE CASCADE",
        "ALTER TABLE alert_notifications ALTER CONSTRAINT alert_notifications_schedule_id_fkey
            DEFERRABLE INITIALLY DEFERRED",
        "ALTER TABLE alert_email_attempts DROP CONSTRAINT alert_email_attempts_sequence_check;
            ALTER TABLE alert_email_attempts ADD CONSTRAINT alert_email_attempts_sequence_check CHECK(true)",
        "ALTER TABLE alert_preferences ADD CONSTRAINT unexpected CHECK(true)",
        "DROP INDEX alert_inbox_order;
            CREATE INDEX alert_inbox_order ON alert_notifications(recipient,created_seconds DESC,created_nanos,id)",
        "DROP INDEX alert_outbox_due;
            CREATE INDEX alert_outbox_due ON alert_email_outbox(status,next_seconds,next_nanos,id)
                WHERE status='pending'",
        "DROP INDEX alert_schedule_subject;
            CREATE INDEX alert_schedule_subject ON alert_schedule(kind,subject_id,recipient) INCLUDE(status)",
        "ALTER FUNCTION alert_time_valid(bigint,integer) VOLATILE",
        "ALTER FUNCTION alert_payload_valid(bytea,bytea) SECURITY DEFINER",
        "ALTER FUNCTION alert_optional_time_valid(bigint,integer) STRICT",
        "ALTER FUNCTION alert_time_valid(bigint,integer) PARALLEL UNSAFE",
        "CREATE OR REPLACE FUNCTION alert_payload_valid(payload bytea,digest bytea)
            RETURNS boolean LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $$ SELECT true $$",
        "ALTER TABLE alert_schedule DISABLE TRIGGER alert_lock",
        "DROP TRIGGER alert_update ON alert_notifications;
            CREATE TRIGGER alert_update BEFORE UPDATE ON alert_notifications
                FOR EACH ROW WHEN(false) EXECUTE FUNCTION validate_alert_update()",
        "DROP TRIGGER alert_attempt_insert ON alert_email_attempts",
        "DROP TRIGGER alert_outbox_ledger ON alert_email_outbox;
            CREATE CONSTRAINT TRIGGER alert_outbox_ledger AFTER INSERT OR UPDATE ON alert_email_outbox
                DEFERRABLE INITIALLY IMMEDIATE FOR EACH ROW EXECUTE FUNCTION validate_alert_outbox_ledger()",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        open(&db).unwrap();
        db.admin.batch_execute("BEGIN").unwrap();
        db.admin.batch_execute(alteration)
            .unwrap_or_else(|error| panic!("cannot apply {alteration}: {error}"));
        // Commit the altered catalog so startup on its separate connection sees it.
        db.admin.batch_execute("COMMIT").unwrap();
        let rejected = open(&db).is_err();
        assert!(rejected, "startup accepted {alteration}");
    }
}

#[test]
fn alert_runtime_grants_only_append_and_guarded_column_updates() {
    let Some(db) = Fixture::new() else { return };
    open(&db).unwrap();
    let mut runtime = db.runtime();
    for (table, column) in [
        ("alert_scan_cursor", "next_seconds"),
        ("alert_scan_cursor", "next_nanos"),
        ("alert_subject_state", "dirty"),
        ("alert_schedule", "status"),
        ("alert_notifications", "read_seconds"),
        ("alert_email_outbox", "sequence"),
    ] {
        let allowed: bool = runtime
            .query_one(
                "SELECT has_table_privilege(current_user,$1,'SELECT')
                AND has_column_privilege(current_user,$1,$2,'UPDATE')
                AND NOT has_table_privilege(current_user,$1,'UPDATE')",
                &[&table, &column],
            )
            .unwrap()
            .get(0);
        assert!(allowed, "expected guarded UPDATE({column}) on {table}");
    }
    for table in [
        "alert_preferences",
        "alert_subject_state",
        "alert_schedule",
        "alert_notifications",
        "alert_read_receipts",
        "alert_email_outbox",
        "alert_email_attempts",
    ] {
        let allowed: bool = runtime
            .query_one(
                "SELECT has_table_privilege(current_user,$1,'SELECT')
                AND has_any_column_privilege(current_user,$1,'INSERT')
                AND NOT has_table_privilege(current_user,$1,'INSERT')",
                &[&table],
            )
            .unwrap()
            .get(0);
        assert!(allowed, "expected column INSERT on {table}");
    }
    for forbidden in [
        "UPDATE alert_preferences SET revision=revision",
        "UPDATE alert_read_receipts SET read_seconds=read_seconds",
        "UPDATE alert_email_attempts SET payload=payload",
        "UPDATE alert_notifications SET payload=payload",
        "UPDATE alert_schedule SET trigger_seconds=trigger_seconds",
        "UPDATE alert_scan_cursor SET singleton=singleton",
        "INSERT INTO alert_scan_cursor(singleton,kind,cycle) VALUES(true,0,0)",
        "DELETE FROM alert_notifications",
        "TRUNCATE alert_email_attempts",
        "ALTER TABLE alert_schedule DISABLE TRIGGER USER",
        "ALTER FUNCTION alert_time_valid(bigint,integer) VOLATILE",
    ] {
        let error = runtime.batch_execute(forbidden).unwrap_err();
        assert_eq!(
            error.code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE),
            "unexpected rejection for {forbidden}: {error}"
        );
    }
}

#[test]
fn alert_startup_rejects_missing_excess_and_public_grants() {
    for alteration in [
        "REVOKE SELECT ON alert_notifications FROM ROLE",
        "REVOKE INSERT(payload_digest) ON alert_preferences FROM ROLE",
        "REVOKE UPDATE(next_nanos) ON alert_scan_cursor FROM ROLE",
        "REVOKE EXECUTE ON FUNCTION alert_payload_valid(bytea,bytea) FROM ROLE",
        "GRANT SELECT ON alert_notifications TO ROLE WITH GRANT OPTION",
        "GRANT INSERT ON alert_schedule TO ROLE",
        "GRANT UPDATE ON alert_subject_state TO ROLE",
        "GRANT INSERT(singleton) ON alert_scan_cursor TO ROLE",
        "GRANT UPDATE(recipient) ON alert_notifications TO ROLE",
        "GRANT UPDATE(payload) ON alert_preferences TO ROLE",
        "GRANT REFERENCES(id) ON alert_email_outbox TO ROLE",
        "GRANT EXECUTE ON FUNCTION validate_alert_update() TO ROLE",
        "GRANT SELECT ON alert_preferences TO PUBLIC",
        "GRANT UPDATE(read_seconds) ON alert_notifications TO PUBLIC",
        "GRANT EXECUTE ON FUNCTION alert_time_valid(bigint,integer) TO PUBLIC",
        "GRANT EXECUTE ON FUNCTION validate_alert_attempt() TO PUBLIC",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        open(&db).unwrap();
        let alteration = alteration.replace("ROLE", &db.role);
        db.admin.batch_execute(&alteration).unwrap();
        assert!(open(&db).is_err(), "startup accepted {alteration}");
    }
}

#[test]
fn alert_startup_rejects_forbidden_authority_from_inherited_or_settable_roles() {
    for inheritance in ["INHERIT", "NOINHERIT"] {
        let Some(mut db) = Fixture::new() else { return };
        open(&db).unwrap();
        let parent = format!("alert_parent_{}", Uuid::new_v4().simple());
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
                "GRANT UPDATE(payload) ON alert_notifications TO {parent}"
            ))
        })();
        let rejected = altered.is_ok() && open(&db).is_err();
        let cleaned = db.control.batch_execute(&format!(
            "REVOKE UPDATE(payload) ON {}.alert_notifications FROM {parent};
                REVOKE {parent} FROM {}; DROP ROLE {parent}",
            db.schema, db.role
        ));
        altered.unwrap();
        cleaned.unwrap();
        assert!(
            rejected,
            "startup accepted alert authority via {inheritance}"
        );
        open(&db).unwrap();
    }
}
