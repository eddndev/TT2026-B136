use super::*;
use application::ApplicationError;
use serde_json::{json, Value};

mod chronology;
mod direct;

fn open(db: &Fixture) -> Result<PostgresMeasureDecisionStore, ApplicationError> {
    PostgresMeasureDecisionStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn group(
    db: &Fixture,
    actor: &Principal,
    command: &MeasureDecisionCommand,
) -> MeasureDecisionGroupCapture {
    let review = service(db, actor.clone())
        .prepare("session", db.case, no_change(command))
        .unwrap();
    prepare_measure_decision_capture(
        &RingSha256Hasher,
        actor,
        db.case,
        review.command,
        review.material,
    )
    .unwrap()
    .into_group_capture(&RingSha256Hasher, db.at)
    .unwrap()
}

#[test]
fn precautionary_anchor_migration_preserves_rows_and_expanded_constraint_identity() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let stored = persist(&db, seed.seed.actor.clone(), no_change(&seed.command));
    let before = snapshot(&mut db);
    let constraints = |db: &mut Fixture| -> Value {
        db.admin.query_one("SELECT jsonb_agg(jsonb_build_array(oid,conname,pg_get_constraintdef(oid)) ORDER BY oid)
            FROM pg_constraint WHERE conrelid='case_measure_decisions'::regclass", &[]).unwrap().get(0)
    };
    let original = constraints(&mut db);

    db.migrate();

    assert_eq!(constraints(&mut db), original);
    assert_eq!(snapshot(&mut db), before);
    reopened(&db, &seed.seed.actor, &stored);
}

#[test]
fn startup_requires_the_exact_precautionary_selectors_shape_and_foreign_key() {
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    for column in ["anchor_precautionary_hearing_id", "anchor_capture_digest"] {
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE case_measure_decisions ALTER COLUMN {column} SET NOT NULL"
            ))
            .unwrap();
        assert!(open(&db).is_err(), "accepted nonnullable {column}");
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE case_measure_decisions ALTER COLUMN {column} DROP NOT NULL"
            ))
            .unwrap();
        open(&db).unwrap();
    }
    db.admin.batch_execute("ALTER TABLE case_measure_decisions ALTER COLUMN anchor_capture_digest SET DEFAULT decode(repeat('00',32),'hex')").unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute(
            "ALTER TABLE case_measure_decisions ALTER COLUMN anchor_capture_digest DROP DEFAULT",
        )
        .unwrap();
    open(&db).unwrap();
    for name in [
        "measure_decision_anchor_shape",
        "measure_decision_anchor_precautionary",
    ] {
        let definition: String = db
            .admin
            .query_one(
                "SELECT pg_get_constraintdef(oid) FROM pg_constraint
             WHERE conrelid='case_measure_decisions'::regclass AND conname=$1",
                &[&name],
            )
            .unwrap()
            .get(0);
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE case_measure_decisions DROP CONSTRAINT {name}"
            ))
            .unwrap();
        if name == "measure_decision_anchor_shape" {
            db.admin
                .batch_execute(&format!(
                    "ALTER TABLE case_measure_decisions ADD CONSTRAINT {name} CHECK(TRUE)"
                ))
                .unwrap();
        }
        assert!(open(&db).is_err(), "accepted altered {name}");
        if name == "measure_decision_anchor_shape" {
            db.admin
                .batch_execute(&format!(
                    "ALTER TABLE case_measure_decisions DROP CONSTRAINT {name}"
                ))
                .unwrap();
        }
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE case_measure_decisions ADD CONSTRAINT {name} {definition}"
            ))
            .unwrap();
        open(&db).unwrap();
    }
}

#[test]
fn startup_requires_the_precautionary_anchor_trigger_and_exact_function_body() {
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    db.admin
        .batch_execute(
            "ALTER TABLE case_measure_decisions DISABLE TRIGGER measure_decision_hearing_anchor",
        )
        .unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute(
            "ALTER TABLE case_measure_decisions ENABLE TRIGGER measure_decision_hearing_anchor",
        )
        .unwrap();
    open(&db).unwrap();
    let trigger: String = db.admin.query_one(
        "SELECT pg_get_triggerdef(oid) FROM pg_trigger WHERE tgrelid='case_measure_decisions'::regclass
         AND tgname='measure_decision_hearing_anchor'", &[],
    ).unwrap().get(0);
    db.admin
        .batch_execute("DROP TRIGGER measure_decision_hearing_anchor ON case_measure_decisions")
        .unwrap();
    assert!(open(&db).is_err());
    db.admin.batch_execute(&trigger).unwrap();
    open(&db).unwrap();
    let function: String = db
        .admin
        .query_one(
            "SELECT pg_get_functiondef('enforce_measure_decision_hearing_anchor()'::regprocedure)",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin.batch_execute("CREATE OR REPLACE FUNCTION enforce_measure_decision_hearing_anchor()
        RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$").unwrap();
    assert!(open(&db).is_err());
    db.admin.batch_execute(&function).unwrap();
    open(&db).unwrap();
}

#[test]
fn precautionary_anchor_runtime_has_explicit_insert_and_no_delegation_or_function_execute() {
    let Some(mut db) = Fixture::new() else { return };
    for column in ["anchor_precautionary_hearing_id", "anchor_capture_digest"] {
        let row = db
            .admin
            .query_one(
                "SELECT has_column_privilege($1,'case_measure_decisions',$2,'INSERT'),
             has_column_privilege($1,'case_measure_decisions',$2,'UPDATE'),
             has_table_privilege($1,'case_measure_decisions','INSERT')",
                &[&db.role, &column],
            )
            .unwrap();
        assert!(row.get::<_, bool>(0));
        assert!(!row.get::<_, bool>(1));
        assert!(!row.get::<_, bool>(2));
        db.admin
            .batch_execute(&format!(
                "REVOKE INSERT({column}) ON case_measure_decisions FROM {}",
                db.role
            ))
            .unwrap();
        assert!(open(&db).is_err());
        db.admin
            .batch_execute(&format!(
                "GRANT INSERT({column}) ON case_measure_decisions TO {}",
                db.role
            ))
            .unwrap();
        open(&db).unwrap();
        db.admin
            .batch_execute(&format!(
                "GRANT INSERT({column}) ON case_measure_decisions TO {} WITH GRANT OPTION",
                db.role
            ))
            .unwrap();
        assert!(open(&db).is_err());
        db.admin
            .batch_execute(&format!(
                "REVOKE GRANT OPTION FOR INSERT({column}) ON case_measure_decisions FROM {}",
                db.role
            ))
            .unwrap();
        open(&db).unwrap();
    }
    db.admin
        .batch_execute(&format!(
            "GRANT EXECUTE ON FUNCTION enforce_measure_decision_hearing_anchor() TO {}",
            db.role
        ))
        .unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute(&format!(
            "REVOKE EXECUTE ON FUNCTION enforce_measure_decision_hearing_anchor() FROM {}",
            db.role
        ))
        .unwrap();
    open(&db).unwrap();
}

#[test]
fn direct_sql_precautionary_selectors_reject_mixed_missing_wrong_and_absent_material() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let group = group(&db, &seed.seed.actor, &seed.command);
    let original = direct::row(&group);
    let before = snapshot(&mut db);
    direct::insert(&db, &group, &original).expect("the exact P selector is accepted");
    assert_eq!(snapshot(&mut db), before);
    for field in [
        "kind", "missing", "capture", "length", "revision", "hearing", "mixed",
    ] {
        let mut row = original.clone();
        match field {
            "kind" => row["anchor_kind"] = json!("none"),
            "missing" => row["anchor_capture_digest"] = Value::Null,
            "capture" => row["anchor_capture_digest"] = json!(format!("\\x{}", "a1".repeat(32))),
            "length" => row["anchor_capture_digest"] = json!("\\x00"),
            "revision" => row["anchor_revision"] = json!(2),
            "hearing" => row["anchor_precautionary_hearing_id"] = json!(uuid::Uuid::new_v4()),
            "mixed" => row["anchor_hearing_id"] = json!(uuid::Uuid::new_v4()),
            _ => unreachable!(),
        }
        assert!(
            direct::insert(&db, &group, &row).is_err(),
            "accepted altered {field}"
        );
        assert_eq!(snapshot(&mut db), before);
    }
}
