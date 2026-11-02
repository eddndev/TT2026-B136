//! The version guard and the request lock on documents in PostgreSQL: an
//! open or completed request freezes its version against seals and individual
//! signatures, and creating a request races a seal only through committed
//! rows: docs/adr/0086-bilateral-contract-signing.md.
#![allow(dead_code)]
mod case_administration_support;
mod contract_storage_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::documents::CaseDocumentStore as _;
use contract_storage_support::{flow::Request, rejection, Harness};
use domain::{clock::Clock as _, crypto::DocumentVersion};
use infrastructure::PostgresCaseDocumentStore;
use postgres::Client;
use std::time::{Duration, Instant};
use uuid::Uuid;

const SEAL: &str =
    "UPDATE documents SET evidence='{\"sealed\":true}'::jsonb WHERE id=$1 AND version=1";

fn frozen() -> (String, String) {
    ("23514".into(), "contract version is frozen".into())
}

fn seal(client: &mut Client, test: &Harness) -> Result<u64, postgres::Error> {
    client.execute(SEAL, &[&test.individual.record.id.as_uuid()])
}

/// A real individual signature of the contracted version, through its store.
fn sign_individually(test: &mut Harness) -> bool {
    use application::document_signing::IndividualSigningStore as _;
    let command = test.individual.command(Uuid::new_v4());
    test.individual.store().commit(command).is_ok()
}

fn evidence_and_receipts(test: &mut Harness) -> (bool, i64) {
    let document = test.individual.record.id.as_uuid();
    let row = test
        .db()
        .admin
        .query_one(
            "SELECT (SELECT evidence IS NULL FROM documents WHERE version=1 AND id=$1),
            (SELECT count(*) FROM individual_document_signatures)",
            &[&document],
        )
        .unwrap();
    (row.get(0), row.get(1))
}

#[test]
fn an_open_or_completed_request_freezes_its_version() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    let mut runtime = test.runtime();
    assert_eq!(rejection(seal(&mut runtime, &test)), frozen());
    assert!(!sign_individually(&mut test));
    assert_eq!(evidence_and_receipts(&mut test), (true, 0));
    test.sign_client(&request, request.created() + 60).unwrap();
    test.sign_firm(&request, request.created() + 120).unwrap();
    assert_eq!(rejection(seal(&mut runtime, &test)), frozen());
    assert!(!sign_individually(&mut test));
    assert_eq!(evidence_and_receipts(&mut test), (true, 0));
}

#[test]
fn a_closed_request_releases_its_version() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    let client = test.client.clone();
    test.close(
        &request,
        "rejected",
        Some(&client),
        None,
        request.created() + 60,
    )
    .unwrap();
    assert!(sign_individually(&mut test));
    assert_eq!(evidence_and_receipts(&mut test), (false, 1));
}

#[test]
fn an_exhausted_request_does_not_freeze_its_version() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    // The guard reads the database clock, and the triggers never admit a
    // request created before the bindings it fixes, so an already exhausted
    // request is seeded past them.
    let past: i64 = test
        .db()
        .admin
        .query_one("SELECT extract(epoch FROM now())::bigint-3*86400", &[])
        .unwrap()
        .get(0);
    let request = test.prepare(past, 1);
    let mut admin = Client::connect(&test.individual.db.admin_url, postgres::NoTls).unwrap();
    let mut tx = admin.transaction().unwrap();
    tx.batch_execute("SET LOCAL session_replication_role=replica")
        .unwrap();
    test.insert_request_row(&mut tx, &request).unwrap();
    tx.commit().unwrap();
    let mut runtime = test.runtime();
    assert_eq!(seal(&mut runtime, &test).unwrap(), 1);
}

/// Evidence that the version guard admitted: it reads the database clock,
/// so a seal after `expires_at` commits while a signature accepted before
/// `expires_at` by the application clock is still on its way.
fn admitted_seal(test: &Harness) {
    let mut admin = Client::connect(&test.individual.db.admin_url, postgres::NoTls).unwrap();
    let mut tx = admin.transaction().unwrap();
    tx.batch_execute("SET LOCAL session_replication_role=replica")
        .unwrap();
    tx.execute(SEAL, &[&test.individual.record.id.as_uuid()])
        .unwrap();
    tx.commit().unwrap();
}

#[test]
fn no_signature_follows_evidence_on_its_version() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    admitted_seal(&test);
    assert_eq!(
        rejection(test.sign_client(&request, request.created() + 60)),
        frozen()
    );

    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    test.sign_client(&request, request.created() + 60).unwrap();
    admitted_seal(&test);
    assert_eq!(
        rejection(test.sign_firm(&request, request.created() + 120)),
        frozen()
    );
    let completed: i64 = test
        .db()
        .admin
        .query_one("SELECT count(*) FROM contract_completions", &[])
        .unwrap()
        .get(0);
    assert_eq!(completed, 0);
}

#[test]
fn signing_a_superseded_version_is_rejected() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    let record = individual_document_backend_support::processor::processor()
        .prepare_version(
            test.individual.record.id,
            DocumentVersion::new(2).unwrap(),
            "reviewed.pdf",
            b"%PDF-1.7\n% second version\n%%EOF\n",
        )
        .unwrap();
    PostgresCaseDocumentStore::open(&test.individual.db.runtime_url)
        .unwrap()
        .append(
            test.individual.actor.id,
            test.individual.db.case,
            DocumentVersion::new(1).unwrap(),
            record,
            test.individual.clock.now(),
        )
        .unwrap();
    assert_eq!(
        rejection(test.sign_client(&request, request.created() + 60)),
        ("23514".into(), "contract version is superseded".into())
    );
}

fn pid(client: &mut Client) -> i32 {
    client
        .query_one("SELECT pg_backend_pid()", &[])
        .unwrap()
        .get(0)
}

/// Waits until `pid` blocks on a lock held by the other transaction.
fn wait_for_lock(test: &mut Harness, pid: i32) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let waiting: bool = test
            .db()
            .admin
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock')",
                &[&pid],
            )
            .unwrap()
            .get(0);
        if waiting {
            return;
        }
        assert!(Instant::now() < deadline, "the second writer never blocked");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_seal_waiting_on_a_new_request_sees_it_and_is_rejected() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    let (revision, sequence) = requested_audit(&mut test, &request);
    let mut first = test.runtime();
    let mut tx = first.transaction().unwrap();
    test.insert_request_row(&mut tx, &request).unwrap();
    let mut second = test.runtime();
    let blocked = pid(&mut second);
    let document = test.individual.record.id.as_uuid();
    let sealing = std::thread::spawn(move || {
        second
            .execute(SEAL, &[&document])
            .map_err(|error| error.as_db_error().map(|e| e.message().to_owned()))
    });
    wait_for_lock(&mut test, blocked);
    requested_event(&mut test, &mut tx, &request, revision, sequence);
    tx.commit().unwrap();
    assert_eq!(
        sealing.join().unwrap(),
        Err(Some("contract version is frozen".to_owned()))
    );
}

#[test]
fn a_request_waiting_on_a_seal_sees_it_and_is_rejected() {
    let Some(test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    let mut first = test.runtime();
    let mut tx = first.transaction().unwrap();
    tx.execute(SEAL, &[&test.individual.record.id.as_uuid()])
        .unwrap();
    let url = test.individual.db.runtime_url.clone();
    let mut second = Client::connect(&url, postgres::NoTls).unwrap();
    let blocked = pid(&mut second);
    let shared = std::sync::Arc::new(std::sync::Mutex::new(test));
    let worker = shared.clone();
    let requesting = std::thread::spawn(move || {
        let test = worker.lock().unwrap();
        let mut tx = second.transaction().unwrap();
        test.insert_request_row(&mut tx, &request)
            .map_err(|error| error.as_db_error().map(|e| e.message().to_owned()))
    });
    std::thread::sleep(Duration::from_millis(50));
    let mut watcher = Client::connect(&url, postgres::NoTls).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while !watcher
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock')",
            &[&blocked],
        )
        .unwrap()
        .get::<_, bool>(0)
    {
        assert!(Instant::now() < deadline, "the request never blocked");
        std::thread::sleep(Duration::from_millis(20));
    }
    tx.commit().unwrap();
    assert_eq!(
        requesting.join().unwrap(),
        Err(Some(
            "contract document version is not available".to_owned()
        ))
    );
}

fn requested_audit(test: &mut Harness, request: &Request) -> (i64, i64) {
    let revision = test.next_revision();
    let firm = test.firm();
    let sequence = test.audit(
        &firm.email,
        "contract.requested",
        &contract_storage_support::flow::resource(request.id, revision),
        request.created(),
    );
    (revision, sequence)
}

fn requested_event(
    test: &mut Harness,
    tx: &mut postgres::Transaction<'_>,
    request: &Request,
    revision: i64,
    sequence: i64,
) {
    let firm = test.firm();
    contract_storage_support::flow::insert_event(
        tx,
        &contract_storage_support::flow::Event {
            contract: test.contract,
            revision,
            kind: "requested",
            request: Some(request.id),
            actor: Some(&firm),
            reason: None,
            at: request.created(),
            operation: Uuid::new_v4(),
            sequence,
        },
    )
    .unwrap();
}
