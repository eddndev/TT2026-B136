use crate::audit_query_support::*;

#[test]
fn exact_fractional_order_ties_boundaries_and_snapshot_exclude_backdated_appends() {
    let Some(mut db) = Fixture::new() else { return };
    let start = db.at.replace_nanosecond(0).unwrap();
    let first = append(&db, "actor", "op", "r", start + Duration::nanoseconds(1));
    let zero = append(&db, "actor", "op", "r", start);
    let tied = append(&db, "actor", "op", "r", start + Duration::nanoseconds(1));
    let boundary = append(&db, "actor", "op", "r", start + Duration::nanoseconds(2));
    let request = AuditEventQuery::new(
        start,
        start + Duration::nanoseconds(2),
        None,
        None,
        None,
        2,
        None,
    )
    .unwrap();
    let adapter = store(&db);
    let page = adapter
        .read(&owner(&db), &request, start + Duration::nanoseconds(1))
        .unwrap();
    assert_eq!(page.snapshot_max_sequence, Some(boundary));
    assert_eq!(
        page.events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![zero, first]
    );
    assert!(page.has_more);
    let cursor = request
        .cursor_after(boundary, page.events.last().unwrap())
        .unwrap();
    let later = append(&db, "actor", "op", "r", start + Duration::nanoseconds(1));
    assert!(later > boundary);
    let next = AuditEventQuery::new(
        start,
        start + Duration::nanoseconds(2),
        None,
        None,
        None,
        2,
        Some(&cursor),
    )
    .unwrap();
    let page = adapter
        .read(&owner(&db), &next, start + Duration::nanoseconds(1))
        .unwrap();
    assert_eq!(page.snapshot_max_sequence, Some(boundary));
    assert_eq!(
        page.events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![tied]
    );
    assert!(!page.has_more);
    assert_eq!(count(&mut db), 7);
    assert_chain(&db);
}

#[test]
fn exact_filters_run_before_limit_and_oversized_unselected_text_stays_unloaded() {
    let Some(db) = Fixture::new() else { return };
    append(&db, &"x".repeat(262145), "ignore", "ignore", db.at);
    append(&db, "same", "wanted", "other", db.at);
    append(&db, "other", "wanted", "same", db.at);
    append(&db, "same", "other", "same", db.at);
    let selected = append(&db, "same", "wanted", "same", db.at);
    let request = AuditEventQuery::new(
        db.at - Duration::seconds(1),
        db.at + Duration::seconds(1),
        Some("same"),
        Some("wanted"),
        Some("same"),
        1,
        None,
    )
    .unwrap();
    let page = store(&db).read(&owner(&db), &request, db.at).unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].sequence, selected);
    assert!(!page.has_more);
    let read = trail(&db).pop().unwrap();
    assert_eq!(read.event.action, "audit.events_read");
    assert_eq!(read.event.actor, "owner@example.test");
    assert_chain(&db);
}

#[test]
fn empty_history_snapshot_and_denied_principal_never_leak_or_add_read_events() {
    let Some(mut db) = Fixture::new() else { return };
    let adapter = store(&db);
    let page = adapter
        .read(&owner(&db), &query(&db, 20, None), db.at)
        .unwrap();
    assert_eq!(page.snapshot_max_sequence, None);
    assert!(page.events.is_empty() && !page.has_more);
    let before = count(&mut db);
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let mut actor = owner(&db);
        actor.role = role;
        assert!(adapter.read(&actor, &query(&db, 20, None), db.at).is_err());
    }
    let mut changed = owner(&db);
    changed.email = "stale@example.test".into();
    assert!(adapter
        .read(&changed, &query(&db, 20, None), db.at)
        .is_err());
    db.user("owner", false);
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&db.owner.as_uuid()]).unwrap();
    assert!(adapter
        .read(&owner(&db), &query(&db, 20, None), db.at)
        .is_err());
    assert_eq!(count(&mut db), before);
}

#[test]
fn full_principal_is_reloaded_after_waiting_for_an_audited_account_change() {
    let Some(mut db) = Fixture::new() else { return };
    append(&db, "actor", "op", "r", db.at);
    let adapter = store(&db);
    let actor = owner(&db);
    let request = query(&db, 20, None);
    let at = db.at;
    let before = count(&mut db);
    db.admin
        .batch_execute("SELECT pg_advisory_lock(280603412820)")
        .unwrap();
    let handle = std::thread::spawn(move || adapter.read(&actor, &request, at));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let waiting: bool = db.admin.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')", &[&db.role]).unwrap().get(0);
        if waiting {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "query never waited for mutation lock"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    db.admin
        .execute(
            "UPDATE users SET email='changed@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("SELECT pg_advisory_unlock(280603412820)")
        .unwrap();
    assert!(matches!(
        handle.join().unwrap(),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(count(&mut db), before);
}

#[test]
fn byte_capacity_rejects_whole_page_but_not_an_unreturned_lookahead() {
    let Some(mut db) = Fixture::new() else { return };
    append(&db, "first", "op", "r", db.at);
    append(&db, &"x".repeat(262145), "op", "r", db.at);
    let adapter = store(&db);
    let page = adapter
        .read(&owner(&db), &query(&db, 1, None), db.at)
        .unwrap();
    assert_eq!(page.events.len(), 1);
    assert!(page.has_more);
    let before = count(&mut db);
    assert!(matches!(
        adapter.read(&owner(&db), &query(&db, 2, None), db.at),
        Err(ApplicationError::AuditQueryCapacityExceeded)
    ));
    assert_eq!(count(&mut db), before);
}

#[test]
fn failed_audit_append_rolls_back_successful_selection() {
    let Some(mut db) = Fixture::new() else { return };
    append(&db, "actor", "op", "r", db.at);
    let adapter = store(&db);
    let before = original_columns(&mut db);
    db.admin
        .batch_execute(
            "CREATE FUNCTION fail_query_audit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.action='audit.events_read' THEN RAISE EXCEPTION 'forced audit failure'; END IF;
        RETURN NEW; END $$; CREATE TRIGGER fail_query_audit BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION fail_query_audit()",
        )
        .unwrap();
    assert!(adapter
        .read(&owner(&db), &query(&db, 20, None), db.at)
        .is_err());
    assert_eq!(original_columns(&mut db), before);
}

#[test]
fn continuation_above_current_head_and_damaged_projection_are_rejected_without_audit() {
    let Some(mut db) = Fixture::new() else { return };
    append(&db, "actor", "op", "r", db.at);
    let adapter = store(&db);
    let request = query(&db, 20, None);
    let event = trail(&db).remove(0).event;
    let cursor = request.cursor_after(500, &event).unwrap();
    let before = count(&mut db);
    assert!(adapter
        .read(&owner(&db), &query(&db, 20, Some(&cursor)), db.at)
        .is_err());
    db.admin
        .batch_execute(
            "ALTER TABLE audit_events ALTER COLUMN timestamp_nanos DROP EXPRESSION;
        UPDATE audit_events SET timestamp_nanos=timestamp_nanos+1",
        )
        .unwrap();
    assert!(adapter.read(&owner(&db), &request, db.at).is_err());
    assert_eq!(count(&mut db), before);
}
