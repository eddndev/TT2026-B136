use super::*;
use application::ApplicationError;
use serde_json::{json, Value};

mod direct;

fn open(db: &Fixture) -> Result<PostgresMeasureDecisionStore, ApplicationError> {
    PostgresMeasureDecisionStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

#[test]
fn repeated_anchor_migration_preserves_none_and_initial_rows_and_constraint_identity() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let mut unanchored = no_change(&seed.command);
    unanchored.anchor = None;
    let none = persist(&db, seed.actor.clone(), unanchored);
    let initial = persist(&db, seed.actor.clone(), no_change(&seed.command));
    let before = snapshot(&mut db);
    let constraints = |db: &mut Fixture| -> Value {
        db.admin.query_one("SELECT jsonb_agg(jsonb_build_array(oid,conname,pg_get_constraintdef(oid)) ORDER BY oid)
            FROM pg_constraint WHERE conrelid='case_measure_decisions'::regclass", &[]).unwrap().get(0)
    };
    let original_constraints = constraints(&mut db);

    db.migrate();

    assert_eq!(constraints(&mut db), original_constraints);
    assert_eq!(snapshot(&mut db), before);
    reopened(&db, &seed.actor, &none);
    reopened(&db, &seed.actor, &initial);
}

#[test]
fn startup_requires_the_exact_anchor_shape_foreign_key_and_column_defaults() {
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    for (damage, repair) in [
        (
            "ALTER TABLE case_measure_decisions ALTER COLUMN anchor_kind DROP DEFAULT",
            "ALTER TABLE case_measure_decisions ALTER COLUMN anchor_kind SET DEFAULT 'none'",
        ),
        (
            "ALTER TABLE case_measure_decisions ALTER COLUMN anchor_kind DROP NOT NULL",
            "ALTER TABLE case_measure_decisions ALTER COLUMN anchor_kind SET NOT NULL",
        ),
    ] {
        db.admin.batch_execute(damage).unwrap();
        assert!(
            open(&db).is_err(),
            "accepted altered anchor column: {damage}"
        );
        db.admin.batch_execute(repair).unwrap();
        open(&db).unwrap();
    }
    for name in [
        "measure_decision_anchor_shape",
        "measure_decision_anchor_hearing",
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
        assert!(
            open(&db).is_err(),
            "accepted altered anchor constraint: {name}"
        );
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
fn runtime_anchor_selectors_have_only_explicit_nondelegatable_insert_authority() {
    let Some(mut db) = Fixture::new() else { return };
    for column in [
        "anchor_kind",
        "anchor_hearing_id",
        "anchor_revision",
        "anchor_values_digest",
        "anchor_submission_digest",
    ] {
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
    }
    db.admin
        .batch_execute(&format!(
            "REVOKE INSERT(anchor_revision) ON case_measure_decisions FROM {}",
            db.role
        ))
        .unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute(&format!(
            "GRANT INSERT(anchor_revision) ON case_measure_decisions TO {}",
            db.role
        ))
        .unwrap();
    open(&db).unwrap();
    db.admin
        .batch_execute(&format!(
            "GRANT INSERT(anchor_revision) ON case_measure_decisions TO {} WITH GRANT OPTION",
            db.role
        ))
        .unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute(&format!(
            "REVOKE GRANT OPTION FOR INSERT(anchor_revision) ON case_measure_decisions FROM {}",
            db.role
        ))
        .unwrap();
    open(&db).unwrap();
}

#[test]
fn direct_sql_initial_selectors_reject_mixed_missing_wrong_and_absent_material_atomically() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let review = service(&db, seed.actor.clone())
        .prepare("session", db.case, no_change(&seed.command))
        .unwrap();
    let group = prepare_measure_decision_capture(
        &RingSha256Hasher,
        &seed.actor,
        db.case,
        review.command,
        review.material,
    )
    .unwrap()
    .into_group_capture(&RingSha256Hasher, db.at)
    .unwrap();
    let original = direct::row(&group);
    let before = snapshot(&mut db);
    direct::insert(&db, &group, &original)
        .expect("exact Initial selector is the direct-insert control");
    assert_eq!(snapshot(&mut db), before);
    for field in [
        "kind",
        "missing",
        "values",
        "submission",
        "revision",
        "hearing",
    ] {
        let mut row = original.clone();
        match field {
            "kind" => row["anchor_kind"] = json!("none"),
            "missing" => row["anchor_submission_digest"] = Value::Null,
            "values" => row["anchor_values_digest"] = json!(format!("\\x{}", "a1".repeat(32))),
            "submission" => row["anchor_submission_digest"] = json!("\\x00"),
            "revision" => row["anchor_revision"] = json!(2),
            "hearing" => row["anchor_hearing_id"] = json!(uuid::Uuid::new_v4()),
            _ => unreachable!(),
        }
        assert!(
            direct::insert(&db, &group, &row).is_err(),
            "accepted altered selector: {field}"
        );
        assert_eq!(snapshot(&mut db), before);
    }
}
