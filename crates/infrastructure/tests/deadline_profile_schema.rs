mod case_administration_support;
mod deadline_profile_schema_support;
use case_administration_support::Fixture;
use deadline_profile_schema_support::*;
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use serde_json::json;
use uuid::Uuid;

#[test]
fn migration_adds_empty_profile_history_and_preserves_existing_data_when_repeated() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["deadline_profiles", "deadline_profile_revisions"] {
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
        assert_eq!(count, 0, "migration must not invent a profile");
    }
    let before = db.snapshot();
    db.migrate();
    assert_eq!(db.snapshot(), before);
    open(&db).unwrap();
}

#[test]
fn profile_projection_preserves_global_and_case_scope_and_receipt_parser_is_strict() {
    let Some(mut db) = Fixture::new() else { return };
    for case in [None, Some(db.case)] {
        let bytes = definition(case);
        let view: serde_json::Value = db
            .admin
            .query_one("SELECT deadline_profile_definition($1)", &[&bytes])
            .unwrap()
            .get(0);
        assert_eq!(view["title"], "Synthetic rule");
        assert_eq!(view.as_object().unwrap().len(), 2);
        if let Some(case) = case {
            assert_eq!(
                view["scope"],
                json!({"kind":"case","case_id":case.to_string()})
            );
        } else {
            assert_eq!(view["scope"]["kind"], "global");
            assert_eq!(view["scope"]["value"]["entity_codes"], json!(["09"]));
            assert_eq!(view["scope"]["value"]["authority"], "Synthetic authority");
        }
        let digest = RingSha256Hasher.hash_bytes(&bytes);
        let bytes = receipt(
            db.owner.as_uuid(),
            Uuid::nil(),
            Uuid::nil(),
            0,
            0,
            digest.as_bytes(),
            None,
        );
        let view: serde_json::Value = db
            .admin
            .query_one("SELECT deadline_profile_submission($1)", &[&bytes])
            .unwrap()
            .get(0);
        assert_eq!(
            view,
            json!({"actor_id":db.owner.to_string(),"operation_id":Uuid::nil(),"profile_id":Uuid::nil(),"action":"publish","expected_revision":0,"algorithm":1,"definition_digest":digest.to_hex(),"reason":null})
        );
        for length in [0, 5, 53, 91] {
            assert!(db
                .admin
                .query_one(
                    "SELECT deadline_profile_submission($1)",
                    &[&&bytes[..length]]
                )
                .is_err());
        }
        for (position, value) in [(0, 0), (53, 3), (58, 0), (91, 1)] {
            let mut bad = bytes.clone();
            bad[position] = value;
            assert!(db
                .admin
                .query_one("SELECT deadline_profile_submission($1)", &[&bad])
                .is_err());
        }
        let mut bad = bytes;
        bad.push(0);
        assert!(db
            .admin
            .query_one("SELECT deadline_profile_submission($1)", &[&bad])
            .is_err());
    }
}

#[test]
fn sql_guards_preserve_initial_scope_contiguous_history_and_terminal_retirement() {
    let Some(mut db) = Fixture::new() else { return };
    let id = Uuid::nil();
    let bytes = definition(None);
    append(&mut db, id, None, 1, "publish", &bytes).unwrap();
    assert!(append(&mut db, id, None, 3, "replace", &bytes).is_err());
    assert!(append(
        &mut db,
        id,
        None,
        2,
        "replace",
        &altered_global_definition()
    )
    .is_err());
    assert!(append(&mut db, id, None, 2, "retire", &altered_global_definition()).is_err());
    append(&mut db, id, None, 2, "replace", &bytes).unwrap();
    append(&mut db, id, None, 3, "retire", &bytes).unwrap();
    assert!(append(&mut db, id, None, 4, "replace", &bytes).is_err());
    for sql in [
        "UPDATE deadline_profiles SET id=id",
        "DELETE FROM deadline_profile_revisions",
        "TRUNCATE deadline_profiles CASCADE",
    ] {
        assert_eq!(
            db.admin.batch_execute(sql).unwrap_err().code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
    let case = db.case;
    assert!(append(&mut db, Uuid::new_v4(), Some(case), 1, "publish", &bytes).is_err());
    let actor = db.owner.as_uuid();
    db.admin
        .execute("UPDATE users SET active=false WHERE id=$1", &[&actor])
        .unwrap();
    assert!(append(&mut db, Uuid::new_v4(), None, 1, "publish", &bytes).is_err());
}

#[test]
fn startup_rejects_changed_profile_columns_functions_constraints_and_trigger_bodies() {
    for alteration in [
        "ALTER FUNCTION deadline_profile_definition(bytea) SECURITY DEFINER",
        "ALTER FUNCTION deadline_profile_submission(bytea) VOLATILE",
        "ALTER FUNCTION deadline_profile_definition(bytea) RESET search_path",
        "CREATE OR REPLACE FUNCTION enforce_deadline_profile_sequence() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$",
        "ALTER TABLE deadline_profile_revisions ALTER COLUMN operation_id DROP NOT NULL",
        "ALTER TABLE deadline_profile_revisions ADD COLUMN unrecognized text",
        "ALTER TABLE deadline_profiles ENABLE ROW LEVEL SECURITY",
        "ALTER TABLE deadline_profiles ALTER CONSTRAINT deadline_profile_first_revision NOT DEFERRABLE",
        "ALTER TABLE deadline_profile_revisions DROP CONSTRAINT deadline_profile_definition_hash; ALTER TABLE deadline_profile_revisions ADD CONSTRAINT deadline_profile_definition_hash CHECK(true)",
        "ALTER TABLE deadline_profile_revisions DROP CONSTRAINT deadline_profile_operation_unique",
        "ALTER TABLE deadline_profile_revisions ALTER COLUMN definition_view DROP EXPRESSION",
        "ALTER TABLE deadline_profile_revisions DISABLE TRIGGER deadline_profile_sequence",
    ] {
        let Some(mut db)=Fixture::new() else {return}; open(&db).unwrap();
        db.admin.batch_execute(alteration).unwrap();
        assert!(open(&db).is_err(),"startup accepted {alteration}");
    }
}

#[test]
fn runtime_requires_append_and_helpers_and_rejects_direct_or_inherited_rewrite_authority() {
    let Some(mut db) = Fixture::new() else { return };
    for table in ["deadline_profiles", "deadline_profile_revisions"] {
        let row=db.admin.query_one("SELECT has_table_privilege($1,$2,'SELECT'),has_table_privilege($1,$2,'INSERT'),has_table_privilege($1,$2,'UPDATE,DELETE,TRUNCATE,TRIGGER')",&[&db.role,&table]).unwrap();
        assert!(row.get::<_, bool>(0));
        assert!(row.get::<_, bool>(1));
        assert!(!row.get::<_, bool>(2));
    }
    for alteration in [
        "GRANT UPDATE ON deadline_profile_revisions TO",
        "GRANT UPDATE(definition_digest) ON deadline_profile_revisions TO",
        "GRANT EXECUTE ON FUNCTION enforce_deadline_profile_sequence() TO",
        "ALTER TABLE deadline_profiles OWNER TO",
        "ALTER FUNCTION deadline_profile_definition(bytea) OWNER TO",
        "REVOKE EXECUTE ON FUNCTION deadline_profile_submission(bytea) FROM",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        db.admin
            .batch_execute(&format!("{alteration} {}", db.role))
            .unwrap();
        assert!(open(&db).is_err(), "startup accepted {alteration}");
    }
    for inherit in ["INHERIT", "NOINHERIT"] {
        let Some(mut db) = Fixture::new() else { return };
        let parent = format!("profile_parent_{}", Uuid::new_v4().simple());
        db.control
            .batch_execute(&format!(
                "CREATE ROLE {parent} NOLOGIN; GRANT {parent} TO {}; ALTER ROLE {} {inherit}",
                db.role, db.role
            ))
            .unwrap();
        db.admin
            .batch_execute(&format!(
                "GRANT DELETE ON deadline_profile_revisions TO {parent}"
            ))
            .unwrap();
        let rejected = open(&db).is_err();
        db.admin
            .batch_execute(&format!(
                "REVOKE ALL ON deadline_profile_revisions FROM {parent}"
            ))
            .unwrap();
        db.control
            .batch_execute(&format!(
                "REVOKE {parent} FROM {}; DROP ROLE {parent}",
                db.role
            ))
            .unwrap();
        assert!(rejected, "indirect authority via {inherit}");
    }
}

#[test]
fn inventory_checks_nil_roots_all_definition_bytes_and_historical_actor_existence() {
    for corruption in ["root_without_revision", "malformed_corpus", "missing_actor"] {
        let Some(mut db) = Fixture::new() else { return };
        let id = Uuid::nil();
        if corruption == "root_without_revision" {
            db.admin.batch_execute("SET session_replication_role=replica; INSERT INTO deadline_profiles(id) VALUES('00000000-0000-0000-0000-000000000000'); SET session_replication_role=origin").unwrap();
        } else {
            if corruption == "missing_actor" {
                db.owner = db.user("owner", false);
            }
            let mut bytes = definition(None);
            if corruption == "malformed_corpus" {
                *bytes.last_mut().unwrap() = 0;
            }
            append(&mut db, id, None, 1, "publish", &bytes).unwrap();
            if corruption == "missing_actor" {
                db.admin
                    .batch_execute("SET session_replication_role=replica")
                    .unwrap();
                db.admin
                    .execute("DELETE FROM users WHERE id=$1", &[&db.owner.as_uuid()])
                    .unwrap();
                db.admin
                    .batch_execute("SET session_replication_role=origin")
                    .unwrap();
            }
        }
        assert!(open(&db).is_err(), "startup accepted {corruption}");
    }
}

#[test]
fn startup_rejects_inherited_profile_tables_even_when_the_child_is_empty() {
    for parent in ["deadline_profiles", "deadline_profile_revisions"] {
        let Some(mut db) = Fixture::new() else { return };
        open(&db).unwrap();
        db.admin
            .batch_execute(&format!(
                "CREATE TABLE profile_history_child () INHERITS ({parent})"
            ))
            .unwrap();
        assert!(
            open(&db).is_err(),
            "startup accepted an inherited child of {parent}"
        );
    }
}
