use super::*;
use time::{Date, Duration, Month, OffsetDateTime, UtcOffset};

fn invalid_times(at: OffsetDateTime) -> [OffsetDateTime; 3] {
    [
        at.to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
        at - Duration::nanoseconds(1),
    ]
}

#[test]
fn invalid_store_capture_clocks_leave_no_group_member_or_audit_rows() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let draft = service(&db, seed.actor.clone())
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    for at in invalid_times(db.at) {
        let storage = Arc::new(
            PostgresMeasureDecisionStore::open(
                &db.runtime_url,
                Arc::new(RingSha256Hasher),
                Arc::new(FixedClock(at)),
            )
            .unwrap(),
        );
        let workflow = MeasureDecisionService::new(
            storage,
            Arc::new(TestIdentity(seed.actor.clone())),
            processor(),
            Arc::new(FormatCheck(None)),
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        );
        let before = snapshot(&mut db);
        assert!(workflow
            .submit(
                "session",
                db.case,
                seed.command.clone(),
                confirmation(&draft)
            )
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn unsupported_or_regressed_store_read_clocks_do_not_append_access_audits() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    persist(&db, seed.actor.clone(), seed.command.clone());
    for at in invalid_times(db.at) {
        let storage = PostgresMeasureDecisionStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(at)),
        )
        .unwrap();
        let before = snapshot(&mut db);
        assert!(storage
            .get(&seed.actor, db.case, seed.command.decision_id)
            .is_err());
        assert!(storage
            .get_operation(&seed.actor, db.case, seed.command.operation_id)
            .is_err());
        assert!(storage
            .list(&seed.actor, db.case, MeasureDecisionReadQuery::default())
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}
