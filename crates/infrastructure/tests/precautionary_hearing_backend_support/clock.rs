use super::*;

fn non_utc_store(db: &Fixture) -> Arc<PostgresPrecautionaryHearingStore> {
    let at = db.at.to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
    Arc::new(
        PostgresPrecautionaryHearingStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(at)),
        )
        .unwrap(),
    )
}

#[test]
fn a_non_utc_store_clock_cannot_commit_through_a_valid_utc_service() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let draft = service(&db, actor.clone())
        .prepare("session", db.case, command.clone())
        .unwrap();
    let workflow = PrecautionaryHearingService::new(
        non_utc_store(&db),
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    let before = snapshot(&mut db);

    assert!(workflow
        .submit("session", db.case, command, confirmation(&draft),)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn non_utc_store_read_clocks_cannot_append_successful_access_audits() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let stored = persist(&db, actor.clone(), command.clone());
    let store = non_utc_store(&db);
    for method in 0..3 {
        let before = snapshot(&mut db);
        let result = match method {
            0 => PrecautionaryHearingReadStore::get(
                store.as_ref(),
                &actor,
                db.case,
                command.hearing_id,
                None,
            )
            .map(|_| ()),
            1 => PrecautionaryHearingReadStore::get_operation(
                store.as_ref(),
                &actor,
                db.case,
                command.operation_id,
            )
            .map(|_| ()),
            _ => PrecautionaryHearingReadStore::list(
                store.as_ref(),
                &actor,
                db.case,
                PrecautionaryHearingReadQuery::default(),
            )
            .map(|_| ()),
        };
        assert!(result.is_err(), "read method {method}");
        assert_eq!(snapshot(&mut db), before);
    }
    assert_eq!(stored.capture.recorded_at, db.at);
}
