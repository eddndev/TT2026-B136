use super::*;
use application::ApplicationError;
use infrastructure::PostgresCaseRepository;

#[test]
fn startup_rejects_email_intent_before_the_report_is_ready() {
    let Some(mut db) = fixture() else { return };
    let actor = owner(&mut db);
    let store = store(&db, clock(db.at));
    let queued = estimated_request(&mut db, &store, &actor, Some(4000));
    open(&db).unwrap();
    insert_intent(&mut db, queued.id);
    reject_without_changes(&mut db);
    remove_intent(&mut db, queued.id);
    open(&db).unwrap();
}

#[test]
fn startup_rejects_email_for_unknown_or_two_second_estimates() {
    for estimate in [None, Some(2000)] {
        let Some(mut db) = fixture() else { return };
        let actor = owner(&mut db);
        let store = store(&db, clock(db.at));
        let queued = estimated_request(&mut db, &store, &actor, estimate);
        let ready = publish(&store, &queued, db.at);
        open(&db).unwrap();
        insert_intent(&mut db, ready.id);
        reject_without_changes(&mut db);
        remove_intent(&mut db, ready.id);
        open(&db).unwrap();
    }
}

#[test]
fn startup_rejects_an_eligible_ready_report_without_its_email_intent() {
    let Some(mut db) = fixture() else { return };
    let actor = owner(&mut db);
    let store = store(&db, clock(db.at));
    let ready = ready(&mut db, &store, &actor);
    open(&db).unwrap();
    let original = delivery(&mut db, ready.id).unwrap();
    remove_intent(&mut db, ready.id);
    reject_without_changes(&mut db);
    db.admin
        .execute(
            "INSERT INTO case_report_email_deliveries
        SELECT * FROM jsonb_populate_record(NULL::case_report_email_deliveries,$1)",
            &[&original],
        )
        .unwrap();
    open(&db).unwrap();
}

fn open(db: &Fixture) -> Result<PostgresCaseRepository, ApplicationError> {
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher))
}

fn reject_without_changes(db: &mut Fixture) {
    let before = ledger(db);
    assert!(matches!(
        open(db),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    assert_eq!(
        ledger(db),
        before,
        "startup must not repair inconsistent email history"
    );
}

fn insert_intent(db: &mut Fixture, id: CaseReportId) {
    db.admin
        .execute(
            "INSERT INTO case_report_email_deliveries
        (report_id,delivery_id,state,generation,created_at,updated_at)
        VALUES($1,$2,'pending',0,$3,$3)",
            &[
                &id.as_uuid(),
                &Uuid::new_v4(),
                &db.at.format(&Rfc3339).unwrap(),
            ],
        )
        .unwrap();
}

fn remove_intent(db: &mut Fixture, id: CaseReportId) {
    db.admin
        .batch_execute("SET session_replication_role=replica")
        .unwrap();
    db.admin
        .execute(
            "DELETE FROM case_report_email_deliveries WHERE report_id=$1",
            &[&id.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("SET session_replication_role=origin")
        .unwrap();
}
