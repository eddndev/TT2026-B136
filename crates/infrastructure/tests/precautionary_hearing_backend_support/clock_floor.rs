use super::*;
use std::sync::Mutex;

fn earlier_store(db: &Fixture) -> Arc<PostgresPrecautionaryHearingStore> {
    Arc::new(
        PostgresPrecautionaryHearingStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at - Duration::seconds(1))),
        )
        .unwrap(),
    )
}

#[test]
fn utc_read_clock_before_capture_rejects_before_committing_access_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    persist(&db, actor.clone(), command.clone());
    let storage = earlier_store(&db);
    let reader = PrecautionaryHearingReadService::new(
        storage.clone(),
        Arc::new(TestIdentity(actor.clone())),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at - Duration::seconds(1))),
    );
    for direct in [true, false] {
        for method in 0..3 {
            let before = snapshot(&mut db);
            let result = match (direct, method) {
                (true, 0) => storage
                    .get(&actor, db.case, command.hearing_id, None)
                    .map(|_| ()),
                (true, 1) => storage
                    .get_operation(&actor, db.case, command.operation_id)
                    .map(|_| ()),
                (true, _) => storage
                    .list(&actor, db.case, PrecautionaryHearingReadQuery::default())
                    .map(|_| ()),
                (false, 0) => reader
                    .get("session", db.case, command.hearing_id, None)
                    .map(|_| ()),
                (false, 1) => reader
                    .get_operation("session", db.case, command.operation_id)
                    .map(|_| ()),
                (false, _) => reader
                    .list("session", db.case, PrecautionaryHearingReadQuery::default())
                    .map(|_| ()),
            };
            assert!(
                result.is_err(),
                "accepted future capture: direct={direct}, method={method}"
            );
            assert_eq!(
                snapshot(&mut db),
                before,
                "rejected read wrote an access audit"
            );
        }
    }
}

#[test]
fn replay_won_during_admission_rejects_an_earlier_utc_store_clock_without_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let concurrent = service(&db, actor.clone());
    let draft = concurrent
        .prepare("session", db.case, command.clone())
        .unwrap();
    let prior_rows = Arc::new(Mutex::new(None));
    let captured_rows = prior_rows.clone();
    let concurrent_command = command.clone();
    let concurrent_review = draft.clone();
    let case = db.case;
    let url = db.admin_url.clone();
    let workflow = PrecautionaryHearingService::new(
        earlier_store(&db),
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(FormatCheck(Some(Box::new(move || {
            concurrent
                .submit(
                    "session",
                    case,
                    concurrent_command.clone(),
                    confirmation(&concurrent_review),
                )
                .unwrap();
            let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
            let rows: serde_json::Value = client.query_one(
                "SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_precautionary_hearings h),
                 'revisions',(SELECT jsonb_agg(to_jsonb(h) ORDER BY hearing_id,revision) FROM case_precautionary_hearing_revisions h),
                 'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[],
            ).unwrap().get(0);
            *captured_rows.lock().unwrap() = Some(rows);
        })))),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    assert!(workflow
        .submit("session", db.case, command, confirmation(&draft))
        .is_err());
    assert_eq!(
        snapshot(&mut db),
        prior_rows
            .lock()
            .unwrap()
            .clone()
            .expect("concurrent original must commit during admission"),
        "failed replay must preserve the successfully committed original without another audit"
    );
}
