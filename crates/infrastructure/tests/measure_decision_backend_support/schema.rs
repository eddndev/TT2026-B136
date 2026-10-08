use super::*;
use application::ApplicationError;

fn open(db: &Fixture) -> Result<PostgresMeasureDecisionStore, ApplicationError> {
    PostgresMeasureDecisionStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

#[test]
fn startup_rejects_weakened_measure_constraints_and_changed_guard_bodies() {
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    db.admin.batch_execute("ALTER TABLE case_measure_decisions DROP CONSTRAINT measure_decision_values_size; ALTER TABLE case_measure_decisions ADD CONSTRAINT measure_decision_values_size CHECK (octet_length(values_canonical)>0)").unwrap();
    assert!(open(&db).is_err());
    db.admin.batch_execute("ALTER TABLE case_measure_decisions DROP CONSTRAINT measure_decision_values_size; ALTER TABLE case_measure_decisions ADD CONSTRAINT measure_decision_values_size CHECK(octet_length(values_canonical) BETWEEN 80 AND 16076 AND substring(values_canonical FROM 1 FOR 6)=convert_to('MDVAL1','UTF8'))").unwrap();
    open(&db).unwrap();
    for guard in [
        "enforce_measure_decision_capture",
        "enforce_measure_revision_source",
        "enforce_measure_decision_complete",
    ] {
        let definition: String = db
            .admin
            .query_one(
                "SELECT pg_get_functiondef($1::text::regprocedure)",
                &[&format!("{guard}()")],
            )
            .unwrap()
            .get(0);
        db.admin.batch_execute(&format!("CREATE OR REPLACE FUNCTION {guard}() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$")).unwrap();
        assert!(open(&db).is_err(), "accepted changed guard {guard}");
        db.admin.batch_execute(&definition).unwrap();
        open(&db).unwrap();
    }
}

#[test]
fn runtime_measure_authority_is_column_limited_nondelegatable_and_not_public() {
    let Some(mut db) = Fixture::new() else { return };
    for table in [
        "case_measure_operations",
        "case_measure_decisions",
        "case_measures",
        "case_measure_revisions",
    ] {
        let columns: String = db.admin.query_one(
            "SELECT string_agg(quote_ident(attname),',' ORDER BY attnum) FROM pg_attribute WHERE attrelid=$1::text::regclass AND attnum>0 AND NOT attisdropped", &[&table],
        ).unwrap().get(0);
        let grants = db.admin.query_one(
            "SELECT has_table_privilege($1,$2,'SELECT'),has_table_privilege($1,$2,'INSERT'),has_column_privilege($1,$2,'case_id','INSERT')", &[&db.role, &table],
        ).unwrap();
        assert!(grants.get::<_, bool>(0));
        assert!(!grants.get::<_, bool>(1));
        assert!(grants.get::<_, bool>(2));
        for (damage, repair) in [
            (
                format!("GRANT SELECT ON {table} TO PUBLIC"),
                format!("REVOKE SELECT ON {table} FROM PUBLIC"),
            ),
            (
                format!("GRANT UPDATE(case_id) ON {table} TO {}", db.role),
                format!("REVOKE UPDATE(case_id) ON {table} FROM {}", db.role),
            ),
            (
                format!("GRANT INSERT ON {table} TO {}", db.role),
                format!(
                    "REVOKE INSERT ON {table} FROM {}; GRANT INSERT({columns}) ON {table} TO {}",
                    db.role, db.role
                ),
            ),
            (
                format!(
                    "GRANT INSERT(case_id) ON {table} TO {} WITH GRANT OPTION",
                    db.role
                ),
                format!(
                    "REVOKE GRANT OPTION FOR INSERT(case_id) ON {table} FROM {}",
                    db.role
                ),
            ),
        ] {
            db.admin.batch_execute(&damage).unwrap();
            assert!(open(&db).is_err(), "accepted excessive authority: {damage}");
            db.admin.batch_execute(&repair).unwrap();
            open(&db).unwrap();
        }
    }
    db.admin
        .batch_execute("GRANT EXECUTE ON FUNCTION enforce_measure_decision_complete() TO PUBLIC")
        .unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute("REVOKE EXECUTE ON FUNCTION enforce_measure_decision_complete() FROM PUBLIC")
        .unwrap();
    open(&db).unwrap();
}
