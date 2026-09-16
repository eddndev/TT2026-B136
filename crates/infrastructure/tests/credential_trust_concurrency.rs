#[allow(dead_code)]
mod credential_trust_database_support;

use std::{
    sync::{mpsc, Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

use application::{
    credential_trust::{CredentialTrustExpectation, CredentialTrustStore},
    ApplicationError,
};
use infrastructure::PostgresCredentialTrustStore;
use postgres::{Client, NoTls};

use credential_trust_database_support::{materials, Clock, Database};

const AUDIT_LOCK: i64 = 0x4155444954;

#[test]
fn simultaneous_publications_with_the_same_expectation_have_one_success() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let clock = Clock::new(materials().at);
    let stores = [
        PostgresCredentialTrustStore::open(&db.url, clock.clone()).unwrap(),
        PostgresCredentialTrustStore::open(&db.url, clock).unwrap(),
    ];
    let material = materials().inspection(1, -100, 100);
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = stores
        .into_iter()
        .map(|store| {
            let barrier = barrier.clone();
            let material = material.clone();
            thread::spawn(move || {
                barrier.wait();
                store.publish(CredentialTrustExpectation::Absent, material)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(ApplicationError::CredentialTrustRevisionConflict)))
            .count(),
        1
    );
    assert_eq!(db.counts(), (1, 1, 1));
}

#[test]
fn publication_rechecks_time_after_waiting_for_the_audit_lock() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let clock = Clock::new(materials().at);
    let (url, name) = named_url(&db.url);
    let store = PostgresCredentialTrustStore::open(&url, clock.clone()).unwrap();
    let material = materials().inspection(1, -100, 100);
    let mut blocker = Client::connect(&db.url, NoTls).unwrap();
    let mut tx = blocker.transaction().unwrap();
    tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
        .unwrap();
    let (send, recv) = mpsc::channel();
    let handle = thread::spawn(move || {
        send.send(store.publish(CredentialTrustExpectation::Absent, material))
            .unwrap()
    });
    wait_for_lock(&mut db.client, &name);
    clock.set(materials().at + 101);
    tx.commit().unwrap();
    assert!(recv.recv_timeout(Duration::from_secs(5)).unwrap().is_err());
    handle.join().unwrap();
    assert_eq!(db.counts(), (0, 0, 0));
}

#[test]
fn audited_publication_overrides_a_repeatable_read_connection_default() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let (url, name) = named_url(&db.url);
    let mut parsed = reqwest::Url::parse(&url).unwrap();
    parsed
        .query_pairs_mut()
        .clear()
        .append_pair("application_name", &name)
        .append_pair(
            "options",
            &format!(
                "-csearch_path={} -cdefault_transaction_isolation=repeatable\\ read",
                db.schema
            ),
        );
    let parsed = parsed.to_string().replace('+', "%20");
    let store = PostgresCredentialTrustStore::open(&parsed, Clock::new(materials().at)).unwrap();
    let first = materials().inspection(1, -100, 100);
    let mut blocker = Client::connect(&db.url, NoTls).unwrap();
    let mut tx = blocker.transaction().unwrap();
    tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
        .unwrap();
    let handle = thread::spawn(move || store.publish(CredentialTrustExpectation::Absent, first));
    wait_for_lock(&mut db.client, &name);
    tx.commit().unwrap();
    let result = handle.join().unwrap().unwrap();
    assert_eq!(result.revision.get(), 1);
    assert_eq!(db.counts(), (1, 1, 1));
}

#[test]
fn revoked_publisher_privileges_prevent_publication_after_waiting() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let publisher = db.role("GRANT SELECT,INSERT ON participant_credential_authority,participant_credential_trust_revisions,audit_events TO {role}");
    let (url, name) = named_url(&publisher);
    let store = PostgresCredentialTrustStore::open(&url, Clock::new(materials().at)).unwrap();
    let material = materials().inspection(1, -100, 100);
    let mut blocker = Client::connect(&db.url, NoTls).unwrap();
    let mut tx = blocker.transaction().unwrap();
    tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
        .unwrap();
    let handle = thread::spawn(move || store.publish(CredentialTrustExpectation::Absent, material));
    wait_for_lock(&mut db.client, &name);
    db.client
        .batch_execute(&format!(
            "REVOKE INSERT ON participant_credential_trust_revisions FROM {}",
            db.roles[0]
        ))
        .unwrap();
    tx.commit().unwrap();
    let result = handle.join().unwrap();
    assert!(result.is_err(), "{result:?}");
    assert_eq!(db.counts(), (0, 0, 0));
}

#[test]
fn direct_sql_sequence_observes_the_predecessor_committed_during_its_lock_wait() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(1, -100, 100),
        )
        .unwrap();
    const INSERT: &str = "INSERT INTO participant_credential_trust_revisions
        SELECT deployment_id,2,crl_der,crl_digest,2,crl_this_update,crl_next_update,valid_from,
            valid_until,published_at_seconds,published_at_nanoseconds,published_by
        FROM participant_credential_trust_revisions WHERE revision=1";
    let (url, name) = named_url(&db.url);
    let mut waiting = Client::connect(&url, NoTls).unwrap();
    let mut blocker = Client::connect(&db.url, NoTls).unwrap();
    let mut tx = blocker.transaction().unwrap();
    tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
        .unwrap();
    let handle = thread::spawn(move || waiting.batch_execute(INSERT));
    wait_for_lock(&mut db.client, &name);
    tx.batch_execute(INSERT).unwrap();
    tx.commit().unwrap();
    let error = handle.join().unwrap().unwrap_err();
    assert_eq!(error.code().unwrap().code(), "23514");
    assert_eq!(db.counts(), (1, 2, 1));
}

fn named_url(base: &str) -> (String, String) {
    let name = format!("credential_wait_{}", uuid::Uuid::new_v4().simple());
    let mut url = reqwest::Url::parse(base).unwrap();
    url.query_pairs_mut().append_pair("application_name", &name);
    (url.to_string(), name)
}

fn wait_for_lock(client: &mut Client, name: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let waiting: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
            WHERE application_name=$1 AND wait_event='advisory')",
                &[&name],
            )
            .unwrap()
            .get(0);
        if waiting {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("publisher did not wait on the audited transaction lock");
}
