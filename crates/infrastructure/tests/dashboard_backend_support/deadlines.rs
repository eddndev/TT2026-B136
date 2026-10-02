use super::*;

#[test]
fn operational_deadline_boundaries_ignore_retired_heads_but_not_case_closure() {
    let Some(db) = fixture() else { return };
    let (_, deadline) = accepted(&db);
    let due = deadline.calculation.result.due_at().unwrap();
    for (at, overdue, hours, week) in [
        (due - Duration::days(7), 0, 0, 0),
        (due - Duration::days(7) + Duration::nanoseconds(1), 0, 0, 1),
        (due - Duration::hours(48), 0, 0, 1),
        (
            due - Duration::hours(48) + Duration::nanoseconds(1),
            0,
            1,
            1,
        ),
        (due, 0, 1, 1),
        (due + Duration::nanoseconds(1), 1, 0, 0),
    ] {
        let value = store(&db, at).read(db.owner).unwrap();
        assert_eq!(
            (
                value.deadlines_overdue,
                value.deadlines_due_48h,
                value.deadlines_due_7d
            ),
            (overdue, hours, week)
        );
        assert_eq!(value.deadlines_unresolved, 0);
        assert_eq!(value.checked_at, at);
    }
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let closed = store(&db, due).read(db.owner).unwrap();
    assert_eq!(closed.active_cases, 1);
    assert_eq!(closed.deadlines_due_48h, 1);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(2),
            CaseAdministrativeStatus::Active,
            db.at,
        )
        .unwrap();
    dl::persist(
        &dl::service(&db, db.owner, Role::Owner),
        db.case,
        dl::human(dl::retire(&deadline), None),
    );
    let retired = store(&db, due).read(db.owner).unwrap();
    assert_eq!(
        (retired.deadlines_due_48h, retired.deadlines_unresolved),
        (0, 0)
    );
}

#[test]
fn stale_follow_sources_count_as_unresolved_without_using_the_captured_due_date() {
    let Some(mut db) = fixture() else { return };
    let (source, deadline) = accepted(&db);
    let due = deadline.calculation.result.due_at().unwrap();
    procedural_fact_backend_support::persist(
        &procedural_fact_backend_support::service(&db, db.owner, Role::Owner),
        db.case,
        procedural_fact_backend_support::correct(&source),
    );
    let before = dl::snapshot(&mut db);
    let value = store(&db, due).read(db.owner).unwrap();
    assert_eq!(
        (
            value.deadlines_due_48h,
            value.deadlines_due_7d,
            value.deadlines_overdue,
            value.deadlines_unresolved
        ),
        (0, 0, 0, 1)
    );
    let after = dl::snapshot(&mut db);
    assert_eq!(before["revisions"], after["revisions"]);
    assert_eq!(before["events"], after["events"]);
    assert_eq!(audits(&mut db), 1);
}

#[test]
fn failed_audit_rolls_back_and_corrupt_deadline_never_returns_partial_totals() {
    let Some(mut db) = fixture() else { return };
    let (_, deadline) = accepted(&db);
    let dashboard = store(&db, db.at);
    db.admin.batch_execute("CREATE FUNCTION reject_dashboard_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='dashboard.read' THEN RAISE EXCEPTION 'injected dashboard audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_dashboard_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_dashboard_audit()").unwrap();
    let before = dl::snapshot(&mut db);
    assert!(matches!(
        dashboard.read(db.owner),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(dl::snapshot(&mut db), before);
    assert_eq!(audits(&mut db), 0);
    db.admin.batch_execute("DROP TRIGGER reject_dashboard_audit ON audit_events; SET session_replication_role='replica'").unwrap();
    db.admin.execute("UPDATE case_deadline_revisions SET due_at_seconds=due_at_seconds+1 WHERE deadline_id=$1", &[&deadline.id.as_uuid()]).unwrap();
    db.admin
        .batch_execute("SET session_replication_role='origin'")
        .unwrap();
    assert!(dashboard.read(db.owner).is_err());
    assert_eq!(audits(&mut db), 0);
}

#[test]
fn declared_attention_is_not_pending_work_and_dashboard_does_not_change_captures() {
    let Some(mut db) = fixture() else { return };
    let (_, deadline) = accepted(&db);
    let due = deadline.calculation.result.due_at().unwrap();
    dl::persist(
        &dl::service(&db, db.owner, Role::Owner),
        db.case,
        dl::human(dl::attention(&deadline), None),
    );
    let before = dl::snapshot(&mut db);
    let value = store(&db, due + Duration::seconds(1))
        .read(db.owner)
        .unwrap();
    assert_eq!(
        (
            value.deadlines_overdue,
            value.deadlines_due_48h,
            value.deadlines_due_7d,
            value.deadlines_unresolved
        ),
        (0, 0, 0, 0)
    );
    let after = dl::snapshot(&mut db);
    assert_eq!(before["roots"], after["roots"]);
    assert_eq!(before["revisions"], after["revisions"]);
    assert_eq!(before["events"], after["events"]);
    assert_eq!(audits(&mut db), 1);
}
