use crate::{alert_backend_support::atomicity, alert_resource_hearing_support::*};
use application::alerts::*;
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    DomainError,
};
use infrastructure::{PostgresAlertStore, RingSha256Hasher};
use serde_json::{json, Value};
use std::{
    io::Read,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use time::Duration;
use uuid::Uuid;

#[derive(Default)]
struct CountingHasher(AtomicUsize);

impl DocumentHasher for CountingHasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        if data.starts_with(b"RHCR1") {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        RingSha256Hasher.hash_bytes(data)
    }

    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        RingSha256Hasher.hash_stream(reader)
    }
}

fn scan(alerts: &PostgresAlertStore, target: AlertSubject, hasher: &CountingHasher) -> usize {
    for _ in 0..8 {
        hasher.0.store(0, Ordering::SeqCst);
        if matches!(alerts.run_next().unwrap(),
            AlertSchedulerRun::Reconciled { subject, .. } if subject == target)
        {
            return hasher.0.load(Ordering::SeqCst);
        }
    }
    panic!("resource hearing scan did not complete within the fixture budget")
}

fn rescan(db: &mut Fixture) {
    db.admin
        .execute(
            "UPDATE alert_subject_state SET dirty=true WHERE kind=2",
            &[],
        )
        .unwrap();
}

fn plans(db: &mut Fixture) -> Value {
    db.admin
        .query_one(
            "SELECT jsonb_agg(to_jsonb(s) ORDER BY id)
        FROM alert_schedule s WHERE kind=2",
            &[],
        )
        .unwrap()
        .get(0)
}

#[test]
fn retained_own_plans_do_not_multiply_full_capture_verification_per_transaction() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    let (_, created) = create(&mut db, due, None);
    db.user("litigator", true);
    db.user("paralegal", true);
    let hasher = Arc::new(CountingHasher::default());
    let alerts = PostgresAlertStore::open(
        &db.runtime_url,
        hasher.clone(),
        Arc::new(MutableClock::new(db.at)),
        None,
    )
    .unwrap();
    let initial = scan(&alerts, subject(&created), &hasher);
    assert!(
        initial > 0,
        "the first scan must verify the captured receipt"
    );
    let saved = plans(&mut db);
    assert_eq!(saved.as_array().unwrap().len(), 6);
    assert!(saved
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["status"] == "planned"));
    rescan(&mut db);
    let retained = scan(&alerts, subject(&created), &hasher);
    assert_eq!(plans(&mut db), saved);
    assert_eq!(
        retained, initial,
        "existing anticipations and recipients must reuse the source verified in this transaction"
    );
}

fn replace_payload(db: &mut Fixture, id: Uuid, payload: &[u8]) {
    db.admin
        .batch_execute("SET session_replication_role=replica")
        .unwrap();
    db.admin
        .execute(
            "UPDATE alert_schedule SET payload=$2,
        payload_digest=pg_catalog.sha256($2) WHERE id=$1",
            &[&id, &payload],
        )
        .unwrap();
    db.admin
        .batch_execute("SET session_replication_role=origin")
        .unwrap();
}

#[test]
fn own_scan_still_verifies_each_plan_and_reloads_evidence_in_the_next_transaction() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    let (_, created) = create(&mut db, due, None);
    let hasher = Arc::new(CountingHasher::default());
    let alerts = PostgresAlertStore::open(
        &db.runtime_url,
        hasher.clone(),
        Arc::new(MutableClock::new(db.at)),
        None,
    )
    .unwrap();
    scan(&alerts, subject(&created), &hasher);
    let row = db
        .admin
        .query_one(
            "SELECT id,payload FROM alert_schedule WHERE kind=2 ORDER BY id LIMIT 1",
            &[],
        )
        .unwrap();
    let id: Uuid = row.get(0);
    let original: Vec<u8> = row.get(1);
    for (field, replacement) in [
        ("/subject/3", json!(Uuid::new_v4())),
        ("/origin/0", json!(2)),
        ("/origin/1", json!("0".repeat(64))),
        ("/kind", json!(["review"])),
        ("/kind/2/0", json!(due.unix_timestamp() + 1)),
        ("/subject_title", json!("Altered hearing title")),
        ("/case_title", json!("Altered historical case title")),
        (
            "/case_reference",
            json!("Altered historical case reference"),
        ),
    ] {
        let mut payload: Value = serde_json::from_slice(&original).unwrap();
        *payload.pointer_mut(field).unwrap() = replacement;
        replace_payload(&mut db, id, &serde_json::to_vec(&payload).unwrap());
        rescan(&mut db);
        let before = atomicity::snapshot(&mut db);
        assert!(alerts.run_next().is_err(), "accepted changed plan {field}");
        assert_eq!(
            atomicity::snapshot(&mut db),
            before,
            "changed state for {field}"
        );
        replace_payload(&mut db, id, &original);
    }
    assert!(scan(&alerts, subject(&created), &hasher) > 0);
    damage(
        &mut db,
        "DELETE FROM audit_events WHERE action='resource_hearing.registered'",
    );
    rescan(&mut db);
    let before = atomicity::snapshot(&mut db);
    assert!(
        alerts.run_next().is_err(),
        "reused evidence from a previous transaction"
    );
    assert_eq!(atomicity::snapshot(&mut db), before);
}
