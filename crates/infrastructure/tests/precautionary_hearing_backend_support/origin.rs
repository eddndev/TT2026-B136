use super::*;
use application::documents::StageSupportReadLimits;

fn existing_service(
    db: &Fixture,
    actor: Principal,
    storage: Arc<PostgresPrecautionaryHearingStore>,
) -> PrecautionaryHearingService {
    PrecautionaryHearingService::new(
        storage,
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn remove_rows(db: &mut Fixture, hearing: PrecautionaryHearingId, remove_root: bool) {
    let audit = snapshot(db)["audit"].clone();
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute(
        "ALTER TABLE case_precautionary_hearings DISABLE TRIGGER ALL;
         ALTER TABLE case_precautionary_hearing_revisions DISABLE TRIGGER ALL",
    )
    .unwrap();
    tx.execute(
        "DELETE FROM case_precautionary_hearing_revisions
         WHERE hearing_id=$1 AND ($2 OR revision>1)",
        &[&hearing.as_uuid(), &remove_root],
    )
    .unwrap();
    if remove_root {
        tx.execute(
            "DELETE FROM case_precautionary_hearings WHERE id=$1",
            &[&hearing.as_uuid()],
        )
        .unwrap();
    }
    tx.batch_execute(
        "ALTER TABLE case_precautionary_hearings ENABLE TRIGGER ALL;
         ALTER TABLE case_precautionary_hearing_revisions ENABLE TRIGGER ALL",
    )
    .unwrap();
    tx.commit().unwrap();
    assert_eq!(snapshot(db)["audit"], audit);
}

fn exact_confirmation(
    previous: &PrecautionaryHearingStoredOperation,
    command: &PrecautionaryHearingCommand,
    has_predecessor: bool,
) -> PrecautionaryHearingConfirmation {
    let review = &previous.capture.review;
    let checked = prepare_precautionary_hearing_capture(
        &RingSha256Hasher,
        &review.actor,
        review.case_id,
        command.clone(),
        review.observed_context.clone(),
        review.sources.clone(),
        has_predecessor.then_some(&previous.capture),
    )
    .unwrap();
    confirmation(checked.review())
}

#[test]
fn an_open_store_cannot_recreate_a_lost_hearing_under_a_fresh_operation() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let first = persist(&db, actor.clone(), command.clone());
    let storage = store(&db);
    let workflow = existing_service(&db, actor.clone(), storage.clone());
    let mut fresh = command;
    fresh.operation_id = PrecautionaryHearingOperationId::new();
    let expected = exact_confirmation(&first, &fresh, false);
    remove_rows(&mut db, fresh.hearing_id, true);
    let before = snapshot(&mut db);

    let direct = PrecautionaryHearingStore::prepare(
        storage.as_ref(),
        &actor,
        db.case,
        &fresh,
        &StageSupportReadLimits::standard(),
    );
    let prepared = workflow.prepare("session", db.case, fresh.clone());
    let submitted = workflow.submit("session", db.case, fresh, expected);

    assert!(
        direct.is_err(),
        "orphan hearing audit must block preparation"
    );
    assert!(prepared.is_err(), "service must reject the lost origin");
    assert!(
        submitted.is_err(),
        "a new operation cannot replace the origin"
    );
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn an_open_store_cannot_expose_or_replace_an_older_head_after_losing_a_suffix() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let first = persist(&db, actor.clone(), command.clone());
    persist(&db, actor.clone(), replacement(&first));
    let storage = store(&db);
    let workflow = existing_service(&db, actor.clone(), storage.clone());
    let fresh = replacement(&first);
    let expected = exact_confirmation(&first, &fresh, true);
    remove_rows(&mut db, command.hearing_id, false);
    let before = snapshot(&mut db);

    let current = PrecautionaryHearingReadStore::get(
        storage.as_ref(),
        &actor,
        db.case,
        command.hearing_id,
        None,
    );
    let direct = PrecautionaryHearingStore::prepare(
        storage.as_ref(),
        &actor,
        db.case,
        &fresh,
        &StageSupportReadLimits::standard(),
    );
    let prepared = workflow.prepare("session", db.case, fresh.clone());
    let submitted = workflow.submit("session", db.case, fresh, expected);

    assert!(
        current.is_err(),
        "an orphan audit suffix invalidates the current head"
    );
    assert!(
        direct.is_err(),
        "lost suffix must block replacement preparation"
    );
    assert!(
        prepared.is_err(),
        "service must reject the incomplete current history"
    );
    assert!(
        submitted.is_err(),
        "a fresh operation cannot overwrite a lost suffix"
    );
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn an_open_store_cannot_report_an_empty_page_after_losing_a_hearing_root() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    persist(&db, actor.clone(), command.clone());
    let storage = store(&db);
    let workflow = PrecautionaryHearingReadService::new(
        storage.clone(),
        Arc::new(TestIdentity(actor.clone())),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    remove_rows(&mut db, command.hearing_id, true);
    let before = snapshot(&mut db);

    let direct = PrecautionaryHearingReadStore::list(
        storage.as_ref(),
        &actor,
        db.case,
        PrecautionaryHearingReadQuery::default(),
    );
    let result = workflow.list("session", db.case, PrecautionaryHearingReadQuery::default());

    assert!(direct.is_err(), "lost root must not produce an empty page");
    assert!(
        result.is_err(),
        "read service must reject the orphan case audit"
    );
    assert_eq!(snapshot(&mut db), before);
}
