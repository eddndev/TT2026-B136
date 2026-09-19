mod case_administration_support;
mod deadline_schema_support;
#[allow(dead_code)]
mod deadline_tracking_receipt_support;

use case_administration_support::Fixture;
use deadline_schema_support::open;
use deadline_tracking_receipt_support::{assert_projection, register};

const PARSERS: [(&str, &str, &str); 4] = [
    ("deadline_submission_v2(bytea)", "b BYTEA", "JSONB"),
    ("deadline_observations(bytea)", "b BYTEA", "JSONB"),
    ("deadline_tracking(bytea)", "b BYTEA", "JSONB"),
    (
        "deadline_tracking_consistent(bytea,bytea)",
        "tracking_bytes BYTEA, observation_bytes BYTEA",
        "BOOLEAN",
    ),
];

fn require_parsers(db: &mut Fixture) {
    for (signature, _, _) in PARSERS {
        let found: bool = db
            .admin
            .query_one("SELECT to_regprocedure($1) IS NOT NULL", &[&signature])
            .unwrap()
            .get(0);
        assert!(found, "missing parser {signature}");
    }
    open(db).unwrap();
}

#[test]
fn runtime_rejects_reinstated_v1_dispatcher_even_with_an_empty_history() {
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    db.admin
        .batch_execute(include_str!(
            "../../../migrations/0017_deadline_receipts.sql"
        ))
        .unwrap();
    assert!(open(&db).is_err(), "V1-only dispatcher was accepted");
}

#[test]
fn repeated_migration_preserves_dispatcher_oid_and_effective_v2_behavior() {
    let Some(mut db) = Fixture::new() else { return };
    require_parsers(&mut db);
    let oid: u32 = db
        .admin
        .query_one(
            "SELECT 'deadline_submission(bytea)'::regprocedure::oid",
            &[],
        )
        .unwrap()
        .get(0);
    let receipt = register(&db);
    let before = assert_projection(&mut db, &receipt);
    for _ in 0..2 {
        db.migrate();
        require_parsers(&mut db);
        let after: u32 = db
            .admin
            .query_one(
                "SELECT 'deadline_submission(bytea)'::regprocedure::oid",
                &[],
            )
            .unwrap()
            .get(0);
        assert_eq!(oid, after);
        assert_eq!(assert_projection(&mut db, &receipt), before);
    }
}

#[test]
fn runtime_checks_every_new_parser_body_and_execution_metadata() {
    for (signature, arguments, returns) in PARSERS {
        let name = signature.split('(').next().unwrap();
        for attribute in [
            "SECURITY DEFINER",
            "VOLATILE",
            "STRICT",
            "LEAKPROOF",
            "PARALLEL SAFE",
            "RESET search_path",
        ] {
            let Some(mut db) = Fixture::new() else { return };
            require_parsers(&mut db);
            db.admin
                .batch_execute(&format!("ALTER FUNCTION {signature} {attribute}"))
                .unwrap();
            assert!(open(&db).is_err(), "accepted {name}: {attribute}");
        }
        let Some(mut db) = Fixture::new() else { return };
        require_parsers(&mut db);
        db.admin.batch_execute(&format!(
            "CREATE OR REPLACE FUNCTION {name}({arguments}) RETURNS {returns} LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $$ BEGIN RETURN NULL; END; $$"
        )).unwrap();
        assert!(open(&db).is_err(), "accepted replaced parser {name}");
    }
}

#[test]
fn runtime_cannot_own_new_parsers_and_requires_execute_on_each() {
    for (signature, _, _) in PARSERS {
        for alteration in [
            format!("ALTER FUNCTION {signature} OWNER TO"),
            format!("REVOKE EXECUTE ON FUNCTION {signature} FROM"),
        ] {
            let Some(mut db) = Fixture::new() else { return };
            require_parsers(&mut db);
            db.admin
                .batch_execute(&format!("{alteration} {}", db.role))
                .unwrap();
            assert!(open(&db).is_err(), "accepted {alteration}");
        }
    }
}

#[test]
fn runtime_rejects_default_arguments_on_each_new_parser() {
    for (signature, arguments, returns) in PARSERS {
        let name = signature.split('(').next().unwrap();
        let arguments = arguments.replace("BYTEA", "BYTEA DEFAULT NULL");
        let Some(mut db) = Fixture::new() else { return };
        require_parsers(&mut db);
        let body: String = db
            .admin
            .query_one(
                "SELECT prosrc FROM pg_proc WHERE oid=to_regprocedure($1)",
                &[&signature],
            )
            .unwrap()
            .get(0);
        db.admin.batch_execute(&format!(
            "CREATE OR REPLACE FUNCTION {name}({arguments}) RETURNS {returns} LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $body${body}$body$"
        )).unwrap();
        assert!(open(&db).is_err(), "accepted default argument for {name}");
    }
}
