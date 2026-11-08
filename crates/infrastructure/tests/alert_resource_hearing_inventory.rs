use crate::alert_resource_hearing_support::*;
use application::alerts::{AlertSchedulerStore, AlertStore};
use std::sync::Arc;
use time::Duration;

#[test]
fn missing_creation_marker_or_initial_association_denies_existing_alert_reads_and_startup() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    create(&mut db, due, None);
    let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
    drive(&alerts);
    let rows = own_alerts(&alerts, db.owner);
    assert_eq!(rows.len(), 1);
    let id = rows[0].id;
    damage(
        &mut db,
        "DELETE FROM audit_events WHERE action='resource_hearing.registered'",
    );
    assert!(alerts.get(db.owner, id).is_err(), "accepted missing marker");
    assert!(
        alerts.list(db.owner, query(20)).is_err(),
        "listed missing marker"
    );
    assert!(open(&db).is_err(), "restored missing marker");
}

#[test]
fn canonical_but_wrong_parent_origin_revision_or_capture_is_rejected() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    create(&mut db, due, None);
    let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
    drive(&alerts);
    assert_eq!(own_alerts(&alerts, db.owner).len(), 1);
    let row = db
        .admin
        .query_one(
            "SELECT id,payload FROM alert_notifications WHERE kind=2 ORDER BY id LIMIT 1",
            &[],
        )
        .unwrap();
    let id: uuid::Uuid = row.get(0);
    let mut payload: serde_json::Value = serde_json::from_slice(&row.get::<_, Vec<u8>>(1)).unwrap();
    *payload
        .pointer_mut("/subject/3")
        .expect("own payload field") = serde_json::json!(uuid::Uuid::new_v4());
    let payload = serde_json::to_vec(&payload).unwrap();
    db.admin
        .batch_execute("SET session_replication_role=replica")
        .unwrap();
    db.admin
        .execute(
            "UPDATE alert_notifications SET payload=$2,payload_digest=pg_catalog.sha256($2) WHERE id=$1",
            &[&id, &payload],
        )
        .unwrap();
    db.admin
        .batch_execute("SET session_replication_role=origin")
        .unwrap();
    assert!(open(&db).is_err(), "accepted alert_notifications/subject/3");
}

#[test]
fn own_state_parent_and_source_capture_corruption_fail_before_scheduler_writes() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    create(&mut db, due, None);
    let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
    drive(&alerts);
    assert_eq!(own_alerts(&alerts, db.owner).len(), 1);
    let sql = format!(
        "UPDATE alert_subject_state SET resource_id='{}',dirty=true WHERE kind=2",
        uuid::Uuid::new_v4()
    );
    damage(&mut db, &sql);
    let before = data(&mut db);
    assert!(alerts.run_next().is_err(), "scheduled corrupt parent");
    assert_eq!(data(&mut db), before);
    assert!(open(&db).is_err(), "restored corrupt parent");
}
