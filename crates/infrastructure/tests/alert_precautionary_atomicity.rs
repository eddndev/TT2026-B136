use super::*;

fn snapshot(db: &mut Fixture) -> serde_json::Value {
    serde_json::json!({
        "hearings":crate::hearing_fixture::snapshot(db),
        "alerts":crate::alert_backend_support::atomicity::snapshot(db),
    })
}

#[test]
fn failed_precautionary_alert_invalidation_rolls_back_and_exact_replay_keeps_generation() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    let (actor, first) = simple(&mut db, due);
    let id = first.capture.review.command.hearing_id;
    assert_eq!(generation(&mut db, id), 1);
    db.at += Duration::seconds(1);
    let workflow = crate::record_fixture::service(&db, actor);
    let command = replacement(&first, due + Duration::hours(1));
    let review = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    db.admin.batch_execute(&format!("CREATE SEQUENCE precautionary_alert_fault_reached;
        GRANT USAGE ON SEQUENCE precautionary_alert_fault_reached TO {};
        CREATE FUNCTION reject_precautionary_alert_invalidation() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN PERFORM nextval('precautionary_alert_fault_reached');
        RAISE EXCEPTION 'injected precautionary alert invalidation failure'; END; $$;
        CREATE TRIGGER reject_precautionary_alert_invalidation BEFORE INSERT OR UPDATE ON alert_subject_state
        FOR EACH ROW WHEN (NEW.kind=3) EXECUTE FUNCTION reject_precautionary_alert_invalidation()", db.role)).unwrap();
    let before = snapshot(&mut db);
    assert!(workflow
        .submit(
            "session",
            db.case,
            command.clone(),
            crate::hearing_fixture::confirmation(&review)
        )
        .is_err());
    assert!(db
        .admin
        .query_one(
            "SELECT is_called FROM precautionary_alert_fault_reached",
            &[]
        )
        .unwrap()
        .get::<_, bool>(0));
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(generation(&mut db, id), 1);
    db.admin.batch_execute("DROP TRIGGER reject_precautionary_alert_invalidation ON alert_subject_state;
        DROP FUNCTION reject_precautionary_alert_invalidation(); DROP SEQUENCE precautionary_alert_fault_reached").unwrap();
    let second = workflow
        .submit(
            "session",
            db.case,
            command.clone(),
            crate::hearing_fixture::confirmation(&review),
        )
        .unwrap();
    assert_eq!(second.capture.review.result_revision.get(), 2);
    assert_eq!(generation(&mut db, id), 2);
    let replay = workflow
        .submit(
            "session",
            db.case,
            command,
            crate::hearing_fixture::confirmation(&review),
        )
        .unwrap();
    crate::record_fixture::same_operation(&replay, &second);
    assert_eq!(generation(&mut db, id), 2);
    let revisions: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_precautionary_hearing_revisions WHERE hearing_id=$1",
            &[&id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(revisions, 2);
}
