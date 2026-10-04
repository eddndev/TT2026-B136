use crate::case_administration_support::Fixture;
use application::ApplicationError;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;
use uuid::Uuid;

const TABLE: &str = "case_hearing_derived_deadline_origins";

#[test]
fn startup_rejects_added_origin_columns_and_disabled_guards() {
    let Some(mut db) = Fixture::new() else { return };
    require_table(&mut db);
    open(&db).unwrap();
    for (damage, repair) in [
        (
            format!("ALTER TABLE {TABLE} ADD COLUMN extra boolean"),
            format!("ALTER TABLE {TABLE} DROP COLUMN extra"),
        ),
        (
            format!("ALTER TABLE {TABLE} DISABLE TRIGGER USER"),
            format!("ALTER TABLE {TABLE} ENABLE TRIGGER USER"),
        ),
    ] {
        db.admin.batch_execute(&damage).unwrap();
        let rejected = invalid_schema(&db);
        db.admin.batch_execute(&repair).unwrap();
        open(&db).unwrap();
        assert!(
            rejected,
            "startup accepted changed origin catalog: {damage}"
        );
    }
}

#[test]
fn startup_rejects_public_origin_access_and_delegatable_runtime_permissions() {
    let Some(mut db) = Fixture::new() else { return };
    require_table(&mut db);
    open(&db).unwrap();
    for (damage, repair) in [
        (
            format!("GRANT SELECT ON {TABLE} TO PUBLIC"),
            format!("REVOKE SELECT ON {TABLE} FROM PUBLIC"),
        ),
        (
            format!("GRANT INSERT(case_id) ON {TABLE} TO PUBLIC"),
            format!("REVOKE INSERT(case_id) ON {TABLE} FROM PUBLIC"),
        ),
        (
            format!("GRANT SELECT ON {TABLE} TO {} WITH GRANT OPTION", db.role),
            format!("REVOKE GRANT OPTION FOR SELECT ON {TABLE} FROM {}", db.role),
        ),
        (
            format!(
                "GRANT INSERT(case_id) ON {TABLE} TO {} WITH GRANT OPTION",
                db.role
            ),
            format!(
                "REVOKE GRANT OPTION FOR INSERT(case_id) ON {TABLE} FROM {}",
                db.role
            ),
        ),
        (
            format!("GRANT UPDATE(case_id) ON {TABLE} TO {}", db.role),
            format!("REVOKE UPDATE(case_id) ON {TABLE} FROM {}", db.role),
        ),
    ] {
        db.admin.batch_execute(&damage).unwrap();
        let rejected = invalid_schema(&db);
        db.admin.batch_execute(&repair).unwrap();
        open(&db).unwrap();
        assert!(
            rejected,
            "startup accepted unsafe origin permission: {damage}"
        );
    }
}

#[test]
fn runtime_rejects_transitive_set_role_authority_even_with_noinherit() {
    let Some(mut db) = Fixture::new() else { return };
    require_table(&mut db);
    open(&db).unwrap();
    let middle = format!("derived_middle_{}", Uuid::new_v4().simple());
    let parent = format!("derived_parent_{}", Uuid::new_v4().simple());
    db.control
        .batch_execute(&format!(
            "CREATE ROLE {middle} NOLOGIN NOINHERIT NOSUPERUSER NOCREATEROLE;
             CREATE ROLE {parent} NOLOGIN NOINHERIT NOSUPERUSER NOCREATEROLE"
        ))
        .unwrap();
    let setup = (|| -> Result<(), postgres::Error> {
        db.control.batch_execute(&format!(
            "GRANT {parent} TO {middle}; GRANT {middle} TO {};
             ALTER ROLE {} NOINHERIT",
            db.role, db.role
        ))?;
        db.admin
            .batch_execute(&format!("GRANT DELETE ON {TABLE} TO {parent}"))
    })();
    let member = setup.is_ok().then(|| {
        db.admin
            .query_one("SELECT pg_has_role($1,$2,'MEMBER')", &[&db.role, &parent])
            .unwrap()
            .get::<_, bool>(0)
    });
    let rejected = setup.is_ok() && invalid_schema(&db);
    // Remove cluster-wide fixture roles before asserting a rejection.
    let cleanup = db.control.batch_execute(&format!(
        "REVOKE ALL ON {}.{TABLE} FROM {parent};
         REVOKE {middle} FROM {}; REVOKE {parent} FROM {middle};
         ALTER ROLE {} INHERIT; DROP ROLE {middle}; DROP ROLE {parent}",
        db.schema, db.role, db.role
    ));
    setup.unwrap();
    cleanup.unwrap();
    assert!(member.unwrap(), "fixture must expose transitive membership");
    assert!(
        rejected,
        "startup accepted transitive NOINHERIT rewrite authority"
    );
    open(&db).unwrap();
}

#[test]
fn startup_rejects_missing_or_weakened_initial_revision_checks() {
    let Some(mut db) = Fixture::new() else { return };
    require_table(&mut db);
    open(&db).unwrap();
    for (name, column) in [
        ("hearing_derived_deadline_result_initial", "result_revision"),
        (
            "hearing_derived_deadline_deadline_initial",
            "deadline_revision",
        ),
    ] {
        let definition = constraint_definition(&mut db, name);
        assert_eq!(definition, format!("CHECK (({column} = 1))"));
        db.admin
            .batch_execute(&format!("ALTER TABLE {TABLE} DROP CONSTRAINT {name}"))
            .unwrap();
        let missing_rejected = invalid_schema(&db);
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE {TABLE} ADD CONSTRAINT {name} CHECK(true)"
            ))
            .unwrap();
        let weakened_rejected = invalid_schema(&db);
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE {TABLE} DROP CONSTRAINT {name};
                 ALTER TABLE {TABLE} ADD CONSTRAINT {name} {definition}"
            ))
            .unwrap();
        open(&db).unwrap();
        assert!(missing_rejected, "startup accepted absent {name}");
        assert!(weakened_rejected, "startup accepted ineffective {name}");
    }
}

#[test]
fn startup_rejects_a_replaced_origin_guard_and_public_guard_execution() {
    let Some(mut db) = Fixture::new() else { return };
    require_table(&mut db);
    open(&db).unwrap();
    let signature = "enforce_hearing_derived_deadline_origin()";
    let original: String = db
        .admin
        .query_one(
            "SELECT pg_get_functiondef($1::text::regprocedure)",
            &[&signature],
        )
        .unwrap()
        .get(0);
    db.admin
        .batch_execute(&format!(
            "CREATE OR REPLACE FUNCTION {signature} RETURNS trigger
             LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$"
        ))
        .unwrap();
    let replaced_rejected = invalid_schema(&db);
    db.admin.batch_execute(&original).unwrap();
    open(&db).unwrap();
    assert!(
        replaced_rejected,
        "startup accepted an origin guard without validation"
    );
    db.admin
        .batch_execute(&format!("GRANT EXECUTE ON FUNCTION {signature} TO PUBLIC"))
        .unwrap();
    let public_rejected = invalid_schema(&db);
    db.admin
        .batch_execute(&format!(
            "REVOKE EXECUTE ON FUNCTION {signature} FROM PUBLIC"
        ))
        .unwrap();
    open(&db).unwrap();
    assert!(
        public_rejected,
        "startup accepted public origin guard authority"
    );
}

#[test]
fn origins_have_one_compound_slot_per_result_and_deadline_revision() {
    let Some(mut db) = Fixture::new() else { return };
    require_table(&mut db);
    open(&db).unwrap();
    let keys = unique_keys(&mut db, TABLE);
    for (name, expected) in [
        (
            "hearing_derived_deadline_result_unique",
            vec!["result_id".to_owned(), "result_revision".to_owned()],
        ),
        (
            "hearing_derived_deadline_deadline_unique",
            vec!["deadline_id".to_owned(), "deadline_revision".to_owned()],
        ),
    ] {
        assert!(
            keys.contains(&expected),
            "missing exact compound key: {expected:?}"
        );
        let definition = constraint_definition(&mut db, name);
        db.admin
            .batch_execute(&format!("ALTER TABLE {TABLE} DROP CONSTRAINT {name}"))
            .unwrap();
        let rejected = invalid_schema(&db);
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE {TABLE} ADD CONSTRAINT {name} {definition}"
            ))
            .unwrap();
        open(&db).unwrap();
        assert!(
            rejected,
            "startup accepted loss of compound uniqueness: {name}"
        );
    }
}

#[test]
fn compound_origins_do_not_make_ordinary_deadline_sources_globally_unique() {
    let Some(mut db) = Fixture::new() else { return };
    require_table(&mut db);
    open(&db).unwrap();
    let keys = unique_keys(&mut db, "case_deadline_revisions");
    assert!(
        !keys.is_empty(),
        "ordinary deadline identity constraints are absent"
    );
    for key in keys {
        assert!(
            key.iter()
                .any(|column| column == "deadline_id" || column == "operation_id"),
            "a unique source key would forbid separate ordinary deadlines: {key:?}"
        );
    }
}

fn constraint_definition(db: &mut Fixture, name: &str) -> String {
    db.admin
        .query_one(
            "SELECT pg_get_constraintdef(oid) FROM pg_constraint
             WHERE conrelid=$1::text::regclass AND conname=$2",
            &[&TABLE, &name],
        )
        .unwrap()
        .get(0)
}

fn unique_keys(db: &mut Fixture, table: &str) -> Vec<Vec<String>> {
    db.admin
        .query(
            "SELECT ARRAY(SELECT coalesce(a.attname::text,'<expression>')
                FROM unnest(i.indkey) WITH ORDINALITY AS k(attnum,position)
                LEFT JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=k.attnum
                WHERE k.position<=i.indnkeyatts ORDER BY k.position)
             FROM pg_index i WHERE i.indrelid=$1::text::regclass
                AND i.indisunique AND i.indisvalid ORDER BY i.indexrelid",
            &[&table],
        )
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect()
}

fn require_table(db: &mut Fixture) {
    let present: bool = db
        .admin
        .query_one("SELECT to_regclass($1) IS NOT NULL", &[&TABLE])
        .unwrap()
        .get(0);
    assert!(present, "missing hearing-derived deadline origin table");
}

fn invalid_schema(db: &Fixture) -> bool {
    matches!(open(db), Err(ApplicationError::InvalidConfiguration(_)))
}

fn open(db: &Fixture) -> Result<PostgresCaseRepository, ApplicationError> {
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher))
}
