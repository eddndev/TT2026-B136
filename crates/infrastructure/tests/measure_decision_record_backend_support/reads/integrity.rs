use super::*;

#[test]
fn mixed_decision_read_audit_failure_returns_no_receipt_or_partial_page() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    let reader = reads(&db, seed.actor);
    let expected = MeasureDecisionRecordReceipt::V2(Box::new(original.clone()));
    same(
        &reader
            .get("session", db.case, original.origin.decision_id)
            .unwrap(),
        &expected,
    );
    db.admin.batch_execute("CREATE FUNCTION reject_mixed_decision_read() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.action IN ('measure_decision.read','measure_decision.operation','measure_decision.list')
        THEN RAISE EXCEPTION 'injected mixed decision read audit failure'; END IF; RETURN NEW; END; $$;
        CREATE TRIGGER reject_mixed_decision_read BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_mixed_decision_read()").unwrap();
    let before = crate::administrative_fixture::snapshot(&mut db);
    assert!(reader
        .get("session", db.case, original.origin.decision_id)
        .is_err());
    assert!(reader
        .get_operation("session", db.case, original.origin.operation_id)
        .is_err());
    assert!(reader
        .list("session", db.case, MeasureDecisionReadQuery::default())
        .is_err());
    assert_eq!(crate::administrative_fixture::snapshot(&mut db), before);
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_mixed_decision_read ON audit_events;
        DROP FUNCTION reject_mixed_decision_read()",
        )
        .unwrap();
    same(
        &reader
            .get_operation("session", db.case, original.origin.operation_id)
            .unwrap(),
        &expected,
    );
}

#[test]
fn mixed_decision_readers_reject_missing_current_m2_and_its_original_c_ancestry() {
    for statement in [
        "DELETE FROM case_measure_revisions WHERE family='m2'",
        "DELETE FROM case_measure_administrations",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let seed = setup(&mut db);
        let original = persist(&db, seed.actor.clone(), seed.command.clone());
        let reader = reads(&db, seed.actor);
        same(
            &reader
                .get("session", db.case, original.origin.decision_id)
                .unwrap(),
            &MeasureDecisionRecordReceipt::V2(Box::new(original.clone())),
        );
        let tables = [
            "case_measure_operations",
            "case_measure_decisions",
            "case_measure_administrations",
            "case_measures",
            "case_measure_revisions",
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
        let before = crate::administrative_fixture::snapshot(&mut db);
        assert!(reader
            .get("session", db.case, original.origin.decision_id)
            .is_err());
        assert!(reader
            .get_operation("session", db.case, original.origin.operation_id)
            .is_err());
        assert!(reader
            .list("session", db.case, MeasureDecisionReadQuery::default())
            .is_err());
        assert_eq!(crate::administrative_fixture::snapshot(&mut db), before);
    }
}
