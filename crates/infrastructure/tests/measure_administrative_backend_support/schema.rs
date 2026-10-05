use super::*;
use application::ApplicationError;
use serde_json::{json, Value};

mod direct;
mod insertion;

fn open(db: &Fixture) -> Result<PostgresMeasureAdministrativeStore, ApplicationError> {
    PostgresMeasureAdministrativeStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn capture(
    db: &Fixture,
    actor: &Principal,
    judicial: &MeasureDecisionStoredOperation,
    command: MeasureAdministrativeCommand,
) -> MeasureAdministrativeCapture {
    let history = MeasureRecordHistoryEvidence {
        judicial: MeasureHistoryEvidence {
            groups: vec![MeasureGroupEvidence {
                origin: judicial.origin.clone(),
                capture: judicial.group.clone(),
            }],
        },
        administrative: vec![],
    };
    prepare_measure_administrative_record_with_history(
        &RingSha256Hasher,
        actor,
        db.case,
        command,
        judicial.group.review.material.context.clone(),
        &history,
    )
    .unwrap()
    .into_capture(&RingSha256Hasher, db.at)
    .unwrap()
}

#[test]
fn administrative_migration_replay_preserves_both_families_and_real_deferred_owner_constraint() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), command);
    let last = persist(
        &db,
        seed.actor.clone(),
        mark(corrected_reference(&first.capture), seed.command.context),
    );
    let catalog = |db: &mut Fixture| -> Value {
        db.admin.query_one("SELECT jsonb_agg(jsonb_build_array(oid,conname,contype,pg_get_constraintdef(oid)) ORDER BY oid)
            FROM pg_constraint WHERE conrelid IN ('case_measure_operations'::regclass,
                'case_measure_revisions'::regclass,'case_measure_administrations'::regclass)", &[]).unwrap().get(0)
    };
    let before = snapshot(&mut db);
    let constraints = catalog(&mut db);
    let kind: String = db
        .admin
        .query_one(
            "SELECT contype::text FROM pg_constraint
        WHERE conrelid='case_measure_operations'::regclass AND conname='measure_operation_payload'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(kind, "t");

    db.migrate();

    assert_eq!(catalog(&mut db), constraints);
    assert_eq!(snapshot(&mut db), before);
    reopened(&db, &seed.actor, &first);
    reopened(&db, &seed.actor, &last);
    let original = crate::measure_fixture::reads(&db, seed.actor)
        .get("session", db.case, judicial.origin.decision_id)
        .unwrap();
    assert_eq!(original, judicial);
}

#[test]
fn administrative_startup_rejects_changed_shape_target_key_and_shared_validity() {
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    for (table, name, check) in [
        (
            "case_measure_administrations",
            "measure_administration_correction_shape",
            true,
        ),
        (
            "case_measure_administrations",
            "measure_administration_target",
            false,
        ),
        ("case_measure_revisions", "measure_revision_validity", true),
    ] {
        let definition: String = db
            .admin
            .query_one(
                "SELECT pg_get_constraintdef(oid) FROM pg_constraint
            WHERE conrelid=$1::text::regclass AND conname=$2",
                &[&table, &name],
            )
            .unwrap()
            .get(0);
        db.admin
            .batch_execute(&format!("ALTER TABLE {table} DROP CONSTRAINT {name}"))
            .unwrap();
        if check {
            db.admin
                .batch_execute(&format!(
                    "ALTER TABLE {table} ADD CONSTRAINT {name} CHECK(TRUE)"
                ))
                .unwrap();
        }
        assert!(open(&db).is_err(), "accepted altered {name}");
        if check {
            db.admin
                .batch_execute(&format!("ALTER TABLE {table} DROP CONSTRAINT {name}"))
                .unwrap();
        }
        db.admin
            .batch_execute(&format!(
                "ALTER TABLE {table} ADD CONSTRAINT {name} {definition}"
            ))
            .unwrap();
        open(&db).unwrap();
    }
}

#[test]
fn administrative_startup_requires_admission_and_family_aware_deferred_guard_bodies() {
    let Some(mut db) = Fixture::new() else { return };
    for function in [
        "enforce_measure_administration_capture()",
        "enforce_measure_decision_complete()",
    ] {
        let definition: String = db
            .admin
            .query_one(
                "SELECT pg_get_functiondef($1::text::regprocedure)",
                &[&function],
            )
            .unwrap()
            .get(0);
        db.admin
            .batch_execute(&format!(
                "CREATE OR REPLACE FUNCTION {function} RETURNS trigger LANGUAGE plpgsql
            SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$"
            ))
            .unwrap();
        assert!(open(&db).is_err(), "accepted changed {function}");
        db.admin.batch_execute(&definition).unwrap();
        open(&db).unwrap();
    }
    let trigger: String = db
        .admin
        .query_one(
            "SELECT pg_get_triggerdef(oid) FROM pg_trigger
        WHERE tgrelid='case_measure_operations'::regclass AND tgname='measure_operation_payload'",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin
        .batch_execute("DROP TRIGGER measure_operation_payload ON case_measure_operations")
        .unwrap();
    assert!(open(&db).is_err());
    db.admin.batch_execute(&trigger).unwrap();
    open(&db).unwrap();
    db.admin.batch_execute("ALTER TABLE case_measure_administrations DISABLE TRIGGER measure_administration_capture").unwrap();
    assert!(open(&db).is_err());
    db.admin.batch_execute("ALTER TABLE case_measure_administrations ENABLE TRIGGER measure_administration_capture").unwrap();
    open(&db).unwrap();
}

#[test]
fn administrative_runtime_requires_only_explicit_nondelegatable_append_grants() {
    let Some(mut db) = Fixture::new() else { return };
    for statement in [
        format!(
            "GRANT INSERT ON case_measure_administrations TO {}",
            db.role
        ),
        format!(
            "GRANT INSERT(action) ON case_measure_administrations TO {} WITH GRANT OPTION",
            db.role
        ),
        format!(
            "GRANT EXECUTE ON FUNCTION enforce_measure_administration_capture() TO {}",
            db.role
        ),
    ] {
        db.admin.batch_execute(&statement).unwrap();
        assert!(open(&db).is_err(), "accepted {statement}");
        db.migrate();
        open(&db).unwrap();
    }
    db.admin
        .batch_execute(&format!(
            "REVOKE INSERT(validity) ON case_measure_revisions FROM {}",
            db.role
        ))
        .unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute(&format!(
            "GRANT INSERT(validity) ON case_measure_revisions TO {}",
            db.role
        ))
        .unwrap();
    open(&db).unwrap();
}
