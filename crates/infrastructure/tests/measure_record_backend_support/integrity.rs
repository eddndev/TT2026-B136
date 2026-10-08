use super::*;

fn damage(db: &mut Fixture, statement: &str) {
    let mut tx = db.admin.transaction().unwrap();
    for table in [
        "case_measure_administrations",
        "case_measure_revisions",
        "audit_events",
    ] {
        tx.batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.batch_execute(statement).unwrap();
    for table in [
        "case_measure_administrations",
        "case_measure_revisions",
        "audit_events",
    ] {
        tx.batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.commit().unwrap();
}

#[test]
fn already_open_record_readers_fail_closed_after_lost_latest_payload_row_or_audit() {
    for statement in [
        "DELETE FROM case_measure_administrations WHERE action='entered_in_error'",
        "DELETE FROM case_measure_revisions WHERE family='c1' AND revision=3",
        "DELETE FROM audit_events WHERE action='measure_administrative.recorded' AND resource LIKE '%:revision:3:%'",
        "ALTER TABLE case_measure_revisions DROP CONSTRAINT measure_revision_capture_size;
         UPDATE case_measure_revisions SET capture_digest='\\x01' WHERE family='c1' AND revision=3",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let (seed, group, _, marked) = chain(&mut db);
        let expected = administrative(&marked);
        let service = reads(&db, seed.actor);
        same_detail(&service.get("session", db.case, expected.reference.id()).unwrap(), &expected);
        damage(&mut db, statement);
        let before = snapshot(&mut db);
        assert!(service.get("session", db.case, expected.reference.id()).is_err());
        assert!(service.exact("session", db.case, expected.reference).is_err());
        assert!(service.exact("session", db.case, judicial(&group, 0).reference).is_err());
        assert!(service.list("session", db.case, MeasureRecordReadQuery::default()).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn failed_record_read_audit_rolls_back_all_accesses_without_disclosure() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, _, marked) = chain(&mut db);
    let expected = administrative(&marked);
    let service = reads(&db, seed.actor);
    same_detail(
        &service
            .get("session", db.case, expected.reference.id())
            .unwrap(),
        &expected,
    );
    db.admin.batch_execute(
        "CREATE FUNCTION reject_measure_record_read_audit() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN IF NEW.action IN ('measure_record.list','measure_record.read','measure_record.exact')
           THEN RAISE EXCEPTION 'read audit failure'; END IF; RETURN NEW; END; $$;
         CREATE TRIGGER reject_measure_record_read_audit BEFORE INSERT ON audit_events
         FOR EACH ROW EXECUTE FUNCTION reject_measure_record_read_audit()",
    ).unwrap();
    let before = snapshot(&mut db);
    assert!(service
        .get("session", db.case, expected.reference.id())
        .is_err());
    assert!(service
        .exact("session", db.case, expected.reference)
        .is_err());
    assert!(service
        .list("session", db.case, MeasureRecordReadQuery::default())
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_measure_record_read_audit ON audit_events;
        DROP FUNCTION reject_measure_record_read_audit()",
        )
        .unwrap();
    same_detail(
        &service
            .exact("session", db.case, expected.reference)
            .unwrap(),
        &expected,
    );
}

#[test]
fn record_reads_reject_unsupported_and_regressed_store_clocks_without_writes() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, _, marked) = chain(&mut db);
    let expected = administrative(&marked);
    for at in [
        db.at - time::Duration::nanoseconds(1),
        db.at.to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap()),
        db.at.replace_year(0).unwrap(),
    ] {
        let storage = Arc::new(
            PostgresMeasureDecisionStore::open(
                &db.runtime_url,
                Arc::new(RingSha256Hasher),
                Arc::new(FixedClock(at)),
            )
            .unwrap(),
        );
        let service = existing_reads(&db, seed.actor.clone(), storage);
        let before = snapshot(&mut db);
        assert!(service
            .get("session", db.case, expected.reference.id())
            .is_err());
        assert!(service
            .exact("session", db.case, expected.reference)
            .is_err());
        assert!(service
            .list("session", db.case, MeasureRecordReadQuery::default())
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
    same_detail(
        &reads(&db, seed.actor)
            .get("session", db.case, expected.reference.id())
            .unwrap(),
        &expected,
    );
}
