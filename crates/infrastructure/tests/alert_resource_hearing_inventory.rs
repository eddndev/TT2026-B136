use crate::alert_resource_hearing_support::*;
use application::alerts::{AlertSchedulerStore, AlertStore};
use std::sync::Arc;
use time::Duration;

#[test]
fn missing_creation_marker_or_initial_association_denies_existing_alert_reads_and_startup() {
    for missing in ["marker", "association"] {
        let Some(mut db) = Fixture::new() else { return };
        let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
        let (_, created) = create(&mut db, due, None);
        let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
        drive(&alerts);
        let rows = own_alerts(&alerts, db.owner);
        assert_eq!(rows.len(), 1);
        let id = rows[0].id;
        let sql = if missing == "marker" {
            "DELETE FROM audit_events WHERE action='resource_hearing.registered'".into()
        } else {
            format!("DELETE FROM case_resource_activity_association_revisions WHERE association_id='{}' AND revision=1",
                created.origin.association_id)
        };
        damage(&mut db, &sql);
        assert!(
            alerts.get(db.owner, id).is_err(),
            "accepted missing {missing}"
        );
        assert!(
            alerts.list(db.owner, query(20)).is_err(),
            "listed missing {missing}"
        );
        assert!(open(&db).is_err(), "restored missing {missing}");
    }
}

#[test]
fn canonical_but_wrong_parent_origin_revision_or_capture_is_rejected() {
    for (table, field, replacement) in [
        (
            "alert_notifications",
            "/subject/3",
            serde_json::json!(uuid::Uuid::new_v4()),
        ),
        (
            "alert_schedule",
            "/subject/3",
            serde_json::json!(uuid::Uuid::new_v4()),
        ),
        ("alert_notifications", "/origin/0", serde_json::json!(2)),
        (
            "alert_schedule",
            "/origin/1",
            serde_json::json!("0".repeat(64)),
        ),
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
        create(&mut db, due, None);
        let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
        drive(&alerts);
        assert_eq!(own_alerts(&alerts, db.owner).len(), 1);
        let row = db
            .admin
            .query_one(
                &format!("SELECT id,payload FROM {table} WHERE kind=2 ORDER BY id LIMIT 1"),
                &[],
            )
            .unwrap();
        let id: uuid::Uuid = row.get(0);
        let mut payload: serde_json::Value =
            serde_json::from_slice(&row.get::<_, Vec<u8>>(1)).unwrap();
        *payload.pointer_mut(field).expect("own payload field") = replacement;
        let payload = serde_json::to_vec(&payload).unwrap();
        db.admin
            .batch_execute("SET session_replication_role=replica")
            .unwrap();
        db.admin.execute(&format!("UPDATE {table} SET payload=$2,payload_digest=pg_catalog.sha256($2) WHERE id=$1"),
            &[&id, &payload]).unwrap();
        db.admin
            .batch_execute("SET session_replication_role=origin")
            .unwrap();
        assert!(open(&db).is_err(), "accepted {table}{field}");
    }
}

#[test]
fn own_state_parent_and_source_capture_corruption_fail_before_scheduler_writes() {
    for corruption in ["parent", "capture"] {
        let Some(mut db) = Fixture::new() else { return };
        let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
        create(&mut db, due, None);
        let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
        drive(&alerts);
        assert_eq!(own_alerts(&alerts, db.owner).len(), 1);
        let sql = if corruption == "parent" {
            format!(
                "UPDATE alert_subject_state SET resource_id='{}',dirty=true WHERE kind=2",
                uuid::Uuid::new_v4()
            )
        } else {
            "UPDATE case_resource_hearing_revisions SET capture_canonical=capture_canonical||decode('00','hex'), capture_digest=pg_catalog.sha256(capture_canonical||decode('00','hex')); UPDATE alert_subject_state SET dirty=true WHERE kind=2".into()
        };
        damage(&mut db, &sql);
        let before = data(&mut db);
        assert!(alerts.run_next().is_err(), "scheduled corrupt {corruption}");
        assert_eq!(data(&mut db), before);
        assert!(open(&db).is_err(), "restored corrupt {corruption}");
    }
}
