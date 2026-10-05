use super::*;

#[test]
fn precautionary_occurrence_survives_same_date_edit_and_restart_without_family_collision() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    let (actor, first) = simple(&mut db, due);
    let own_case = db.case;
    let id = first.capture.review.command.hearing_id.as_uuid();
    let initial_case = initial_decision(&mut db, due, id);
    let (_, resource) = crate::alert_resource_hearing_support::create(&mut db, due, Some(id));
    let clock = Arc::new(MutableClock::new(due - Duration::hours(12)));
    let alerts = store(&db, clock.clone());
    let mut preferences = AlertPreferenceValues::default();
    preferences.hearing_upcoming.channels = AlertChannels {
        internal: true,
        email: false,
    };
    alerts
        .save_preferences(
            db.owner,
            AlertPreferenceCommand {
                operation_id: operation(),
                expected_revision: 0,
                values: preferences,
            },
        )
        .unwrap();
    drive(&alerts);
    // Dirty subjects can finish ahead of roots first discovered in the next scan cycle.
    clock.set(due - Duration::hours(12) + Duration::seconds(1));
    drive(&alerts);
    let rows = own_alerts(&alerts, db.owner);
    assert_eq!(rows.len(), 1);
    let notification = rows[0].clone();
    assert_eq!(notification.subject, subject(&first));
    assert_eq!(notification.origin, origin(&first));
    assert_eq!(notification.email, AlertEmailStatus::Disabled);
    let all = alerts.list(db.owner, query(20)).unwrap().alerts;
    assert_eq!(all.len(), 3);
    assert_eq!(
        all.iter()
            .filter(|row| row.subject.case_id() == initial_case)
            .count(),
        1
    );
    assert!(all.iter().any(|row| matches!(row.subject, AlertSubject::Hearing { case_id, .. } if case_id == initial_case)));
    assert!(all
        .iter()
        .any(|row| row.subject == crate::alert_resource_hearing_support::subject(&resource)));
    let occurrences = db
        .admin
        .query(
            "SELECT kind,occurrence_key,occurrence_id FROM alert_schedule
        WHERE subject_id=$1 AND status='activated' ORDER BY kind",
            &[&id],
        )
        .unwrap();
    assert_eq!(
        occurrences
            .iter()
            .map(|r| r.get::<_, i16>(0))
            .collect::<Vec<_>>(),
        [0, 2, 3]
    );
    assert_eq!(
        occurrences
            .iter()
            .map(|r| r.get::<_, String>(1))
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
    assert_eq!(
        occurrences
            .iter()
            .map(|r| r.get::<_, uuid::Uuid>(2))
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
    let read = AlertReadCommand {
        operation_id: operation(),
        alert_id: notification.id,
    };
    let receipt = alerts.mark_read(db.owner, read).unwrap();
    db.case = own_case;
    db.at = due - Duration::hours(12) + Duration::seconds(1);
    clock.set(db.at);
    let second = crate::record_fixture::persist(&db, actor, replacement(&first, due));
    assert_eq!(second.capture.review.result_revision.get(), 2);
    drive(&alerts);
    drop(alerts);
    let retained = crate::alert_resource_hearing_support::data(&mut db);
    db.migrate();
    assert_eq!(
        crate::alert_resource_hearing_support::data(&mut db),
        retained
    );
    clock.set(db.at + Duration::seconds(1));
    let reopened = store(&db, clock);
    drive(&reopened);
    let rows = own_alerts(&reopened, db.owner);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, notification.id);
    assert_eq!(rows[0].occurrence_id, notification.occurrence_id);
    assert_eq!(rows[0].origin, notification.origin);
    assert_eq!(rows[0].read_at, receipt.alert.read_at);
    assert_eq!(
        reopened.mark_read(db.owner, read).unwrap().alert.read_at,
        receipt.alert.read_at
    );
}

#[test]
fn mixed_review_alerts_retain_historical_origins_after_rescheduling_and_cancellation() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    let seed = crate::record_fixture::setup(&mut db);
    let mut command = seed.command;
    set_time(&mut command, due);
    let first = crate::record_fixture::persist(&db, seed.actor.clone(), command);
    assert_eq!(
        first.history.record_history.records.judicial.groups.len(),
        1
    );
    assert_eq!(first.history.record_history.records.administrative.len(), 1);
    let clock = Arc::new(MutableClock::new(due - Duration::hours(12)));
    let alerts = store(&db, clock.clone());
    drive(&alerts);
    let rows = own_alerts(&alerts, db.owner);
    assert_eq!(rows.len(), 1);
    let original = rows[0].clone();
    db.at = due - Duration::hours(12) + Duration::seconds(1);
    clock.set(db.at);
    let after = due + Duration::hours(1);
    let second =
        crate::record_fixture::persist(&db, seed.actor.clone(), replacement(&first, after));
    drive(&alerts);
    let historical = alerts.get(db.owner, original.id).unwrap().alert;
    assert_eq!(historical.origin, origin(&first));
    assert!(matches!(
        historical.state,
        AlertState::Resolved {
            reason: AlertResolutionReason::Superseded,
            ..
        }
    ));
    let rows = own_alerts(&alerts, db.owner);
    assert_eq!(rows.len(), 2);
    let current = rows
        .iter()
        .find(|row| row.origin == origin(&second))
        .unwrap()
        .clone();
    assert_eq!(current.state, AlertState::Active);
    assert!(
        matches!(current.kind, AlertKind::Upcoming { activity_at, .. } if activity_at == after)
    );
    db.at += Duration::seconds(1);
    clock.set(db.at);
    let third = crate::record_fixture::persist(&db, seed.actor, cancellation(&second));
    assert_eq!(third.capture.review.result_revision.get(), 3);
    drive(&alerts);
    let resolved = alerts.get(db.owner, current.id).unwrap().alert;
    assert_eq!(resolved.origin, origin(&second));
    assert!(matches!(
        resolved.state,
        AlertState::Resolved {
            reason: AlertResolutionReason::CancelledHearing,
            ..
        }
    ));
    assert_eq!(own_alerts(&alerts, db.owner).len(), 2);
    drop(alerts);
    let reopened = store(&db, clock.clone());
    assert_eq!(
        reopened.get(db.owner, original.id).unwrap().alert,
        historical
    );
    assert_eq!(reopened.get(db.owner, current.id).unwrap().alert, resolved);
    let row = db
        .admin
        .query_one(
            "SELECT payload FROM alert_notifications WHERE id=$1",
            &[&original.id.as_uuid()],
        )
        .unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&row.get::<_, Vec<u8>>(0)).unwrap();
    *payload.pointer_mut("/origin/1").unwrap() = serde_json::json!("0".repeat(64));
    let bytes = serde_json::to_vec(&payload).unwrap();
    db.admin
        .batch_execute("SET session_replication_role=replica")
        .unwrap();
    db.admin.execute("UPDATE alert_notifications SET payload=$2,payload_digest=pg_catalog.sha256($2) WHERE id=$1",
        &[&original.id.as_uuid(), &bytes]).unwrap();
    db.admin
        .batch_execute("SET session_replication_role=origin")
        .unwrap();
    assert!(reopened.get(db.owner, original.id).is_err());
    assert!(open(&db, clock).is_err());
}
