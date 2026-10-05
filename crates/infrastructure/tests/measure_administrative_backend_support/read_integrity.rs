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
fn already_open_readers_reject_lost_payload_latest_record_and_original_audit() {
    for statement in [
        "DELETE FROM case_measure_administrations WHERE action='entered_in_error'",
        "DELETE FROM case_measure_revisions WHERE family='c1' AND revision=4",
        "DELETE FROM audit_events WHERE action='measure_administrative.recorded'
         AND resource LIKE '%:revision:4:%'",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let (seed, originals) = chain(&mut db);
        let service = reads(&db, seed.actor);
        same_operation(
            &service
                .get_operation("session", db.case, originals[2].origin.operation_id)
                .unwrap(),
            &originals[2],
        );
        damage(&mut db, statement);
        let before = snapshot(&mut db);
        for original in &originals {
            assert!(service
                .get_operation("session", db.case, original.origin.operation_id)
                .is_err());
        }
        assert!(service
            .list(
                "session",
                db.case,
                MeasureAdministrativeReadQuery::default()
            )
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn failed_read_audit_insert_rolls_back_disclosure_and_every_durable_row() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), command);
    let service = reads(&db, seed.actor);
    same_operation(
        &service
            .get_operation("session", db.case, original.origin.operation_id)
            .unwrap(),
        &original,
    );
    db.admin.batch_execute(
        "CREATE FUNCTION reject_administrative_read_audit() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
           IF NEW.action IN ('measure_administrative.operation','measure_administrative.list') THEN
             RAISE EXCEPTION 'read audit failure';
           END IF;
           RETURN NEW;
         END; $$;
         CREATE TRIGGER reject_administrative_read_audit BEFORE INSERT ON audit_events
         FOR EACH ROW EXECUTE FUNCTION reject_administrative_read_audit()",
    ).unwrap();
    let before = snapshot(&mut db);
    assert!(service
        .get_operation("session", db.case, original.origin.operation_id)
        .is_err());
    assert!(service
        .list(
            "session",
            db.case,
            MeasureAdministrativeReadQuery::default()
        )
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_administrative_read_audit ON audit_events;
         DROP FUNCTION reject_administrative_read_audit()",
        )
        .unwrap();
    same_operation(
        &service
            .get_operation("session", db.case, original.origin.operation_id)
            .unwrap(),
        &original,
    );
}

#[test]
fn administrative_reads_reject_unsupported_or_regressed_store_clocks_without_writes() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), command);
    let invalid = [
        db.at - time::Duration::nanoseconds(1),
        db.at.to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap()),
        db.at.replace_year(0).unwrap(),
    ];
    for at in invalid {
        let storage = Arc::new(
            PostgresMeasureAdministrativeStore::open(
                &db.runtime_url,
                Arc::new(RingSha256Hasher),
                Arc::new(FixedClock(at)),
            )
            .unwrap(),
        );
        let service = existing_reads(&db, seed.actor.clone(), storage);
        let before = snapshot(&mut db);
        assert!(service
            .get_operation("session", db.case, original.origin.operation_id)
            .is_err());
        assert!(service
            .list(
                "session",
                db.case,
                MeasureAdministrativeReadQuery::default()
            )
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
    same_operation(
        &reads(&db, seed.actor)
            .get_operation("session", db.case, original.origin.operation_id)
            .unwrap(),
        &original,
    );
}
