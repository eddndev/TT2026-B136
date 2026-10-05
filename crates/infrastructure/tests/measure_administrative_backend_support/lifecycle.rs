use super::*;

#[test]
fn actual_judicial_measure_accepts_a_durable_administrative_correction() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let original = &judicial.group.measures[0];

    let stored = persist(&db, seed.actor, command.clone());

    assert_eq!(stored.capture.review.command, command);
    assert_eq!(stored.capture.records.len(), 1);
    assert_eq!(stored.capture.review.result.previous, reference(original));
    assert_eq!(stored.capture.review.result.revision.get(), 2);
    assert_eq!(
        stored.capture.review.result.values.conditions().as_str(),
        "Corrected reporting terms"
    );
    assert_eq!(
        stored.capture.review.result.sources,
        original.result.sources
    );
    assert_eq!(stored.capture.recorded_at, db.at);
    assert_eq!(stored.capture.recorded_at.nanosecond(), 123_456_789);
    assert_eq!(stored.record_history.records.judicial.groups.len(), 1);
    assert_eq!(
        stored.record_history.records.judicial.groups[0].capture,
        judicial.group
    );
    assert_retained(&stored, &judicial, original);
    assert_eq!(
        measure_administrative_origin_with_decision_history(
            &RingSha256Hasher,
            &stored.capture,
            &stored.record_history,
        )
        .unwrap(),
        stored.origin
    );
}

#[test]
fn repeated_corrections_and_mark_preserve_each_original_receipt_and_real_judicial_origin() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), command);
    let second = persist(
        &db,
        seed.actor.clone(),
        correction(
            corrected_reference(&first.capture),
            seed.command.context,
            &first.capture.review.result.values,
            "Second corrected reporting terms",
        ),
    );
    let marked = persist(
        &db,
        seed.actor.clone(),
        mark(corrected_reference(&second.capture), seed.command.context),
    );

    for (operation, revision) in [(&first, 2), (&second, 3), (&marked, 4)] {
        assert_eq!(operation.capture.review.result.revision.get(), revision);
        assert_retained(operation, &judicial, &judicial.group.measures[0]);
        reopened(&db, &seed.actor, operation);
    }
    assert_eq!(
        first.capture.review.result.validity,
        MeasureCaptureValidity::Valid
    );
    assert_eq!(
        second.capture.review.result.validity,
        MeasureCaptureValidity::Valid
    );
    assert_eq!(
        marked.capture.review.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    assert_eq!(
        marked.capture.review.result.values,
        second.capture.review.result.values
    );
    assert_eq!(
        second.capture.review.result.previous,
        corrected_reference(&first.capture)
    );
    assert_eq!(
        marked.capture.review.result.previous,
        corrected_reference(&second.capture)
    );
    assert_eq!(marked.record_history.records.administrative.len(), 2);
    for expected in [&first, &second] {
        let ancestor = marked
            .record_history
            .records
            .administrative
            .iter()
            .find(|a| a.origin.operation_id == expected.origin.operation_id)
            .unwrap();
        assert_eq!(ancestor.origin, expected.origin);
        assert_eq!(ancestor.capture, expected.capture);
    }
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_measures", &[])
            .unwrap()
            .get::<_, i64>(0),
        2
    );
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_measure_administrations", &[])
            .unwrap()
            .get::<_, i64>(0),
        3
    );
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM audit_events WHERE action='measure_administrative.recorded'",
                &[]
            )
            .unwrap()
            .get::<_, i64>(0),
        3
    );
    assert_eq!(
        crate::measure_fixture::reads(&db, seed.actor)
            .get("session", db.case, judicial.origin.decision_id)
            .unwrap(),
        judicial
    );
}

#[test]
fn mark_directly_from_judicial_record_retains_values_and_creates_no_replacement_root() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, _) = setup(&mut db);
    let previous = &judicial.group.measures[0];
    let stored = persist(
        &db,
        seed.actor.clone(),
        mark(reference(previous), seed.command.context),
    );

    assert_retained(&stored, &judicial, previous);
    assert_eq!(stored.capture.review.result.values, previous.result.values);
    assert_eq!(
        stored.capture.review.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    assert!(stored.record_history.records.administrative.is_empty());
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_measures", &[])
            .unwrap()
            .get::<_, i64>(0),
        2
    );
    reopened(&db, &seed.actor, &stored);
}
