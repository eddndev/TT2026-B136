mod case_administration_support;
mod deadline_schema_support;
use case_administration_support::Fixture;
use deadline_schema_support::*;
use serde_json::json;
use uuid::Uuid;

#[test]
fn migration_adds_empty_deadline_history_and_preserves_existing_rows_on_repeat() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["case_deadlines", "case_deadline_revisions"] {
        let exists: bool = db
            .admin
            .query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])
            .unwrap()
            .get(0);
        assert!(exists, "missing {table}");
        let count: i64 = db
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0);
    }
    let before = db.snapshot();
    db.migrate();
    assert_eq!(db.snapshot(), before);
    open(&db).unwrap();
}

#[test]
fn receipt_projection_preserves_targets_and_rejects_noncanonical_envelopes() {
    let Some(mut db) = Fixture::new() else { return };
    for (action, expected, reason, label) in [
        (0, 0, None, "register"),
        (1, 1, Some("Corrected"), "correct"),
        (2, 1, Some("Declared action"), "set_attention"),
        (3, 4294967294, Some("Retired"), "retire"),
    ] {
        let bytes = receipt(
            db.owner.as_uuid(),
            db.case.as_uuid(),
            Uuid::nil(),
            Uuid::nil(),
            action,
            expected,
            reason,
        );
        let value: serde_json::Value = db
            .admin
            .query_one("SELECT deadline_submission($1)", &[&bytes])
            .unwrap()
            .get(0);
        assert_eq!(
            value,
            json!({"actor_id":db.owner.to_string(),"case_id":db.case.to_string(),
            "deadline_id":Uuid::nil(),"operation_id":Uuid::nil(),"action":label,"expected_revision":expected,
            "review_digest":"17".repeat(32),"reason":reason})
        );
        for end in [0, 5, 69, 106] {
            assert!(db
                .admin
                .query_one("SELECT deadline_submission($1)", &[&&bytes[..end]])
                .is_err());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(db
            .admin
            .query_one("SELECT deadline_submission($1)", &[&trailing])
            .is_err());
    }
    for (action, expected, reason) in [
        (4, 0, None),
        (0, 1, None),
        (0, 0, Some("Unexpected")),
        (1, 0, Some("Missing base")),
        (1, 1, None),
        (3, u32::MAX, Some("Overflow")),
    ] {
        let bytes = receipt(
            db.owner.as_uuid(),
            db.case.as_uuid(),
            Uuid::nil(),
            Uuid::nil(),
            action,
            expected,
            reason,
        );
        assert!(db
            .admin
            .query_one("SELECT deadline_submission($1)", &[&bytes])
            .is_err());
    }
}

#[test]
fn attention_json_preserves_precision_and_rejects_unstated_components() {
    let Some(mut db) = Fixture::new() else { return };
    let recorded = |time| json!({"status":"recorded","occurred_at":time,"statement":"Declared filing","locator":"Receipt"});
    for value in [
        json!({"status":"pending"}),
        recorded(json!({"precision":"unknown"})),
        recorded(json!({"precision":"date","year":2026,"month":1,"day":6,"offset_seconds":null})),
        recorded(
            json!({"precision":"minute","year":2026,"month":1,"day":6,"hour":12,"minute":30,"offset_seconds":0}),
        ),
        recorded(
            json!({"precision":"second","year":2026,"month":1,"day":6,"hour":12,"minute":30,"second":45,"offset_seconds":-21600}),
        ),
    ] {
        let valid: bool = db
            .admin
            .query_one("SELECT deadline_attention_valid($1)", &[&value])
            .unwrap()
            .get(0);
        assert!(valid, "{value}");
    }
    for value in [
        json!({"status":"pending","occurred_at":null}),
        json!({"status":"recorded"}),
        recorded(json!({"precision":"unknown","year":2026})),
        recorded(json!({"precision":"date","year":2026,"month":2,"day":30,"offset_seconds":null})),
        recorded(
            json!({"precision":"date","year":2026,"month":1,"day":6,"hour":0,"offset_seconds":null}),
        ),
        recorded(
            json!({"precision":"minute","year":2026,"month":1,"day":6,"hour":12,"minute":30,"second":0,"offset_seconds":null}),
        ),
        recorded(
            json!({"precision":"second","year":2026,"month":1,"day":6,"hour":12,"minute":30,"second":null,"offset_seconds":0}),
        ),
        recorded(json!({"precision":"date","year":1,"month":1,"day":1,"offset_seconds":60})),
        recorded(json!({"precision":"date","year":9999,"month":12,"day":31,"offset_seconds":-60})),
        recorded(
            json!({"precision":"minute","year":2026,"month":1,"day":6,"hour":12,"minute":30,"offset_seconds":61}),
        ),
    ] {
        let valid: bool = db
            .admin
            .query_one("SELECT deadline_attention_valid($1)", &[&value])
            .unwrap()
            .get(0);
        assert!(!valid, "{value}");
    }
}

#[test]
fn root_requires_its_first_revision_and_history_is_statement_immutable() {
    let Some(mut db) = Fixture::new() else { return };
    let mut tx = db.admin.transaction().unwrap();
    tx.execute(
        "INSERT INTO case_deadlines(id,case_id) VALUES($1,$2)",
        &[&Uuid::nil(), &db.case.as_uuid()],
    )
    .unwrap();
    assert!(tx.commit().is_err());
    for sql in [
        "UPDATE case_deadlines SET id=id",
        "DELETE FROM case_deadline_revisions",
        "TRUNCATE case_deadlines CASCADE",
    ] {
        assert_eq!(
            db.admin.batch_execute(sql).unwrap_err().code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}

#[test]
fn startup_rejects_altered_columns_generated_status_constraints_functions_and_triggers() {
    for alteration in [
        "ALTER TABLE case_deadline_revisions ALTER COLUMN capture_digest DROP NOT NULL",
        "ALTER TABLE case_deadline_revisions ADD COLUMN unexpected text",
        "ALTER TABLE case_deadlines ENABLE ROW LEVEL SECURITY",
        "ALTER TABLE case_deadlines ALTER CONSTRAINT deadline_first_revision NOT DEFERRABLE",
        "ALTER TABLE case_deadline_revisions DROP CONSTRAINT deadline_operation_unique",
        "ALTER TABLE case_deadline_revisions DROP CONSTRAINT deadline_capture_hash; ALTER TABLE case_deadline_revisions ADD CONSTRAINT deadline_capture_hash CHECK(true)",
        "ALTER TABLE case_deadline_revisions ALTER COLUMN status DROP EXPRESSION",
        "ALTER FUNCTION deadline_submission(bytea) SECURITY DEFINER",
        "ALTER FUNCTION deadline_attention_valid(jsonb) VOLATILE",
        "ALTER FUNCTION deadline_submission(bytea) RESET search_path",
        "CREATE OR REPLACE FUNCTION enforce_deadline_sequence() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$",
        "ALTER TABLE case_deadline_revisions DISABLE TRIGGER deadline_sequence",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        open(&db).unwrap();
        db.admin.batch_execute(alteration).unwrap();
        assert!(open(&db).is_err(), "startup accepted {alteration}");
    }
}

#[test]
fn runtime_is_append_only_and_rejects_helper_rewrite_authority() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["case_deadlines", "case_deadline_revisions"] {
        let row = db.admin.query_one("SELECT has_table_privilege($1,$2,'SELECT'),has_table_privilege($1,$2,'INSERT'),has_table_privilege($1,$2,'UPDATE,DELETE,TRUNCATE,TRIGGER')", &[&db.role,&table]).unwrap();
        assert!(row.get::<_, bool>(0));
        assert!(row.get::<_, bool>(1));
        assert!(!row.get::<_, bool>(2));
    }
    for alteration in [
        "GRANT UPDATE ON case_deadline_revisions TO",
        "GRANT UPDATE(capture_digest) ON case_deadline_revisions TO",
        "GRANT EXECUTE ON FUNCTION enforce_deadline_sequence() TO",
        "ALTER TABLE case_deadlines OWNER TO",
        "ALTER FUNCTION deadline_submission(bytea) OWNER TO",
        "REVOKE EXECUTE ON FUNCTION deadline_submission(bytea) FROM",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin
            .batch_execute(&format!("{alteration} {}", db.role))
            .unwrap();
        assert!(open(&db).is_err(), "startup accepted {alteration}");
    }
}

#[test]
fn inventory_rejects_an_orphan_nil_root_without_fabricating_a_revision() {
    let Some(mut db) = Fixture::new() else { return };
    db.admin
        .batch_execute("SET session_replication_role=replica")
        .unwrap();
    db.admin
        .execute(
            "INSERT INTO case_deadlines(id,case_id) VALUES($1,$2)",
            &[&Uuid::nil(), &db.case.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("SET session_replication_role=origin")
        .unwrap();
    assert!(open(&db).is_err());
}
