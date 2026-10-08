use super::*;
use application::ApplicationError;
use infrastructure::PostgresMeasureDecisionStore;

fn open(db: &Fixture) -> Result<PostgresMeasureDecisionStore, ApplicationError> {
    PostgresMeasureDecisionStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

#[test]
fn migration_replay_preserves_g1_g2_a_owners_and_matching_member_families() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let stored = persist(&db, seed.actor.clone(), seed.command.clone());
    let before = snapshot(&mut db);
    let constraints = |db: &mut Fixture| -> serde_json::Value {
        db.admin.query_one("SELECT jsonb_agg(jsonb_build_array(oid,conname,contype,pg_get_constraintdef(oid)) ORDER BY oid)
            FROM pg_constraint WHERE conrelid IN ('case_measure_operations'::regclass,
            'case_measure_decisions'::regclass,'case_measure_revisions'::regclass,
            'case_measure_administrations'::regclass)", &[]).unwrap().get(0)
    };
    let original = constraints(&mut db);
    db.migrate();
    assert_eq!(constraints(&mut db), original);
    assert_eq!(snapshot(&mut db), before);
    let pairs: Vec<(String,String)> = db.admin.query("SELECT DISTINCT o.family,r.family
        FROM case_measure_operations o JOIN case_measure_revisions r ON r.owner_operation=o.operation_id
        ORDER BY o.family,r.family", &[]).unwrap().into_iter().map(|r| (r.get(0),r.get(1))).collect();
    assert_eq!(
        pairs,
        vec![
            ("a1".into(), "c1".into()),
            ("g1".into(), "m1".into()),
            ("g2".into(), "m2".into())
        ]
    );
    reopened(&db, &seed.actor, &stored);
    crate::administrative_fixture::reopened(&db, &seed.actor, &seed.corrected);
}

#[test]
fn mixed_startup_rejects_weakened_owner_member_family_and_validity_constraints() {
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    for (table, name) in [
        ("case_measure_operations", "measure_operation_family"),
        ("case_measure_revisions", "measure_revision_family"),
        ("case_measure_revisions", "measure_revision_validity"),
    ] {
        let definition: String = db
            .admin
            .query_one(
                "SELECT pg_get_constraintdef(oid)
            FROM pg_constraint WHERE conrelid=$1::text::regclass AND conname=$2",
                &[&table, &name],
            )
            .unwrap()
            .get(0);
        db.admin.batch_execute(&format!("ALTER TABLE {table} DROP CONSTRAINT {name}; ALTER TABLE {table} ADD CONSTRAINT {name} CHECK(TRUE)")).unwrap();
        assert!(open(&db).is_err(), "accepted weakened {name}");
        db.admin.batch_execute(&format!("ALTER TABLE {table} DROP CONSTRAINT {name}; ALTER TABLE {table} ADD CONSTRAINT {name} {definition}")).unwrap();
        open(&db).unwrap();
    }
}

#[test]
fn mixed_startup_requires_exact_family_dispatch_and_real_deferred_completeness() {
    let Some(mut db) = Fixture::new() else { return };
    for function in [
        "enforce_measure_decision_capture()",
        "enforce_measure_revision_source()",
        "enforce_measure_decision_complete()",
        "enforce_measure_administration_capture()",
    ] {
        let original: String = db
            .admin
            .query_one(
                "SELECT pg_get_functiondef($1::text::regprocedure)",
                &[&function],
            )
            .unwrap()
            .get(0);
        db.admin.batch_execute(&format!("CREATE OR REPLACE FUNCTION {function} RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$")).unwrap();
        assert!(open(&db).is_err(), "accepted changed {function}");
        db.admin.batch_execute(&original).unwrap();
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
}
