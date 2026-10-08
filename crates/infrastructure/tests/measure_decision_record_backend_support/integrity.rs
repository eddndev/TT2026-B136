use super::*;
use application::{documents::StageSupportReadLimits, ApplicationError};
use infrastructure::PostgresMeasureDecisionStore;

fn open(db: &Fixture) -> Result<PostgresMeasureDecisionStore, ApplicationError> {
    PostgresMeasureDecisionStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn damage(db: &mut Fixture, statement: &str) {
    let tables = [
        "case_measure_operations",
        "case_measure_decisions",
        "case_measure_administrations",
        "case_measures",
        "case_measure_revisions",
        "audit_events",
    ];
    let mut tx = db.admin.transaction().unwrap();
    for table in tables {
        tx.batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.batch_execute(statement).unwrap();
    for table in tables {
        tx.batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.commit().unwrap();
}

#[test]
fn lost_g2_payload_member_and_original_audit_fail_on_existing_and_new_connections() {
    for statement in [
        "DELETE FROM case_measure_decisions WHERE operation_id IN (SELECT operation_id FROM case_measure_operations WHERE family='g2')",
        "DELETE FROM case_measure_revisions WHERE family='m2'",
        "DELETE FROM audit_events WHERE sequence=(SELECT audit_sequence FROM case_measure_operations WHERE family='g2')",
        "UPDATE case_measure_decisions SET group_digest=decode(repeat('ab',32),'hex') WHERE operation_id IN (SELECT operation_id FROM case_measure_operations WHERE family='g2')",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let seed = setup(&mut db);
        let stored = persist(&db, seed.actor.clone(), seed.command.clone());
        let storage = crate::measure_fixture::store(&db);
        damage(&mut db, statement);
        let before = snapshot(&mut db);
        assert!(MeasureDecisionRecordStore::prepare(storage.as_ref(), &seed.actor, db.case,
            &stored.group.review.command, &StageSupportReadLimits::standard()).is_err());
        assert!(open(&db).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn g2_reconstruction_validates_unselected_siblings_and_real_c_ancestor_values() {
    for statement in [
        "DELETE FROM case_measure_revisions WHERE family='m1' AND measure_id NOT IN (SELECT target_measure_id FROM case_measure_administrations)",
        "UPDATE case_measure_revisions SET capture_digest=decode(repeat('cd',32),'hex') WHERE family='c1'",
        "DELETE FROM case_measure_administrations",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let seed = setup(&mut db);
        let original = persist(&db, seed.actor.clone(), seed.command.clone());
        let workflow = service(&db, seed.actor.clone());
        damage(&mut db, statement);
        let before = snapshot(&mut db);
        assert!(workflow.prepare("session", db.case, seed.command.clone()).is_err());
        assert!(workflow.submit("session", db.case, seed.command,
            confirmation(&original.group.review)).is_err());
        assert_eq!(snapshot(&mut db), before);
        assert!(open(&db).is_err());
    }
}

#[test]
fn original_g2_audit_blocks_owner_loss_and_cross_family_operation_reuse() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    let storage = crate::measure_fixture::store(&db);
    damage(&mut db, "DELETE FROM case_measure_revisions WHERE family='m2'; DELETE FROM case_measure_decisions WHERE operation_id IN (SELECT operation_id FROM case_measure_operations WHERE family='g2'); DELETE FROM case_measure_operations WHERE family='g2'");
    let before = snapshot(&mut db);
    assert!(MeasureDecisionRecordStore::prepare(
        storage.as_ref(),
        &seed.actor,
        db.case,
        &original.group.review.command,
        &StageSupportReadLimits::standard()
    )
    .is_err());
    let mut reused = crate::measure_fixture::no_change(&seed.command);
    reused.operation_id = original.origin.operation_id;
    assert!(MeasureDecisionStore::prepare(
        storage.as_ref(),
        &seed.actor,
        db.case,
        &reused,
        &StageSupportReadLimits::standard()
    )
    .is_err());
    assert_eq!(snapshot(&mut db), before);
    assert!(open(&db).is_err());
}

#[test]
fn audit_and_deferred_g2_owner_failure_roll_back_all_new_rows_together() {
    for failure in [
        "CREATE FUNCTION reject_g2_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit failure'; END $$; CREATE TRIGGER reject_g2_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_g2_audit()",
        "CREATE FUNCTION reject_g2_owner() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected owner failure'; END $$; CREATE CONSTRAINT TRIGGER reject_g2_owner AFTER INSERT ON case_measure_operations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_g2_owner()",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let seed = setup(&mut db);
        let MeasureDecisionRecordReview::V2(review) = service(&db, seed.actor.clone())
            .prepare("session", db.case, seed.command.clone()).unwrap()
        else { panic!("expected V2 preparation") };
        let url = db.admin_url.clone();
        let workflow = service_with_format(&db, seed.actor, FormatCheck(Some(Box::new(move || {
            postgres::Client::connect(&url, postgres::NoTls).unwrap().batch_execute(failure).unwrap();
        }))));
        let before = snapshot(&mut db);
        assert!(workflow.submit("session", db.case, seed.command, confirmation(&review)).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}
