use super::*;

#[test]
fn mixed_hearing_read_audit_failure_discloses_no_receipt_or_partial_page() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command);
    let reader = reads(&db, seed.actor);
    let id = original.capture.review.command.hearing_id;
    same_operation(
        &reader.get("session", db.case, id, None).unwrap(),
        &original,
    );
    db.admin.batch_execute("CREATE FUNCTION reject_mixed_hearing_read() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.action IN ('precautionary_hearing.read','precautionary_hearing.operation','precautionary_hearing.list')
        THEN RAISE EXCEPTION 'injected mixed hearing read audit failure'; END IF; RETURN NEW; END; $$;
        CREATE TRIGGER reject_mixed_hearing_read BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_mixed_hearing_read()").unwrap();
    let before = snapshot(&mut db);
    assert!(reader.get("session", db.case, id, None).is_err());
    assert!(reader
        .get_operation(
            "session",
            db.case,
            original.capture.review.command.operation_id
        )
        .is_err());
    assert!(reader
        .list("session", db.case, PrecautionaryHearingReadQuery::default())
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_mixed_hearing_read ON audit_events;
        DROP FUNCTION reject_mixed_hearing_read()",
        )
        .unwrap();
    same_operation(
        &reader
            .get_operation(
                "session",
                db.case,
                original.capture.review.command.operation_id,
            )
            .unwrap(),
        &original,
    );
}

#[test]
fn mixed_hearing_readers_reject_a_lost_latest_capture_or_its_selected_m2_proof() {
    for statement in [
        "DELETE FROM case_precautionary_hearing_revisions WHERE action='cancel'",
        "DELETE FROM case_measure_revisions WHERE family='m2'",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let (seed, captures, _) = chain(&mut db);
        let current = &captures[2];
        let id = current.capture.review.command.hearing_id;
        let reader = reads(&db, seed.actor);
        same_operation(&reader.get("session", db.case, id, None).unwrap(), current);
        let tables = [
            "case_precautionary_hearings",
            "case_precautionary_hearing_revisions",
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
        let before = snapshot(&mut db);
        assert!(reader.get("session", db.case, id, None).is_err());
        assert!(reader
            .get_operation(
                "session",
                db.case,
                current.capture.review.command.operation_id
            )
            .is_err());
        assert!(reader
            .list("session", db.case, PrecautionaryHearingReadQuery::default())
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}
