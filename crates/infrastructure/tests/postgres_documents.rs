mod document_store_support;

use application::cases::CaseRepository;
use application::documents::{CaseDocumentStore, DocumentAction};
use application::ApplicationError;
use domain::audit::{verify_chain, ChainVerification};
use domain::identity::Role;
use infrastructure::{PostgresCaseDocumentStore, PostgresCaseRepository, RingSha256Hasher};
use postgres::{Client, NoTls};
use time::OffsetDateTime;

use document_store_support::{case, database_url, document, evidence, user};

// Fixtures install temporary constraints or triggers on shared database tables.
static DATABASE_FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn document_bytes_case_binding_and_audit_timestamp_survive_reconnection() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let actor = user(&url, Role::Litigator);
    let case_id = case(&url, actor);
    let foreign_case = case(&url, owner);
    let record = document();
    let at = OffsetDateTime::from_unix_timestamp_nanos(1_735_689_600_123_456_789).unwrap();
    PostgresCaseDocumentStore::connect(&url)
        .unwrap()
        .insert(actor, case_id, record.clone(), at)
        .unwrap();
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    assert_eq!(
        store
            .load(
                actor,
                case_id,
                record.id,
                application::documents::VersionSelection::Only,
                DocumentAction::Verify
            )
            .unwrap(),
        record
    );
    assert!(matches!(
        store.load(
            actor,
            foreign_case,
            record.id,
            application::documents::VersionSelection::Only,
            DocumentAction::Verify
        ),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    let entries = store.audit_entries(owner).unwrap();
    let entry = entries
        .iter()
        .find(|entry| entry.event.resource.contains(&record.id.to_string()))
        .unwrap();
    assert_eq!(entry.event.timestamp, at);
    assert!(matches!(
        verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
        ChainVerification::Valid { .. }
    ));
}

#[test]
fn revoked_members_and_clients_cannot_commit_or_read_document_evidence() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let actor = user(&url, Role::Litigator);
    let client = user(&url, Role::Client);
    let case_id = case(&url, actor);
    let cases = PostgresCaseRepository::connect(
        &url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap();
    cases
        .add_member(case_id, client, owner, time::OffsetDateTime::now_utc())
        .unwrap();
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let record = document();
    store
        .insert(actor, case_id, record.clone(), OffsetDateTime::now_utc())
        .unwrap();
    for action in [
        DocumentAction::Upload,
        DocumentAction::Seal,
        DocumentAction::Verify,
        DocumentAction::Export,
    ] {
        assert!(matches!(
            store.check_access(client, case_id, action),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    store
        .check_access(actor, case_id, DocumentAction::Seal)
        .unwrap();
    cases
        .remove_member(case_id, actor, owner, time::OffsetDateTime::now_utc())
        .unwrap();
    let mut sealed = record.clone();
    sealed.seal(evidence()).unwrap();
    assert!(store
        .seal(actor, case_id, sealed, OffsetDateTime::now_utc())
        .is_err());
    assert!(store
        .record_access(
            actor,
            case_id,
            &record,
            DocumentAction::Export,
            OffsetDateTime::now_utc()
        )
        .is_err());
    assert!(store
        .load(
            actor,
            case_id,
            record.id,
            application::documents::VersionSelection::Only,
            DocumentAction::Verify
        )
        .is_err());
    assert_eq!(
        store
            .load(
                owner,
                case_id,
                record.id,
                application::documents::VersionSelection::Only,
                DocumentAction::Verify
            )
            .unwrap(),
        record
    );
}

#[test]
fn audit_insert_failure_rolls_back_document_creation() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case_id = case(&url, owner);
    let record = document();
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let constraint = format!("reject_document_{}", record.id.as_uuid().simple());
    let mut admin = Client::connect(&url, NoTls).unwrap();
    admin
        .batch_execute(&format!(
            "ALTER TABLE audit_events ADD CONSTRAINT {constraint} CHECK (resource NOT LIKE '%{}%')",
            record.id
        ))
        .unwrap();
    let result = store.insert(owner, case_id, record.clone(), OffsetDateTime::now_utc());
    admin
        .batch_execute(&format!(
            "ALTER TABLE audit_events DROP CONSTRAINT {constraint}"
        ))
        .unwrap();
    assert!(matches!(result, Err(ApplicationError::Port(_))));
    assert!(matches!(
        store.load(
            owner,
            case_id,
            record.id,
            application::documents::VersionSelection::Only,
            DocumentAction::Verify
        ),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    let count: i64 = admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE resource LIKE $1",
            &[&format!("%{}%", record.id)],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[test]
fn competing_seals_capture_one_evidence_value_and_one_success_event() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case_id = case(&url, owner);
    let record = document();
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    store
        .insert(owner, case_id, record.clone(), OffsetDateTime::now_utc())
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|index| {
            let adapter = PostgresCaseDocumentStore::connect(&url).unwrap();
            let mut candidate = record.clone();
            let mut evidence = evidence();
            evidence.timestamp_token.push(index);
            candidate.seal(evidence).unwrap();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                (
                    adapter.seal(owner, case_id, candidate.clone(), OffsetDateTime::now_utc()),
                    candidate,
                )
            })
        })
        .collect();
    let mut winner = None;
    for worker in workers {
        let (result, candidate) = worker.join().unwrap();
        match result {
            Ok(()) => assert!(winner.replace(candidate).is_none()),
            Err(ApplicationError::DocumentAlreadySealed(_)) => {}
            other => panic!("unexpected seal result: {other:?}"),
        }
    }
    assert_eq!(
        store
            .load(
                owner,
                case_id,
                record.id,
                application::documents::VersionSelection::Only,
                DocumentAction::Verify
            )
            .unwrap(),
        winner.unwrap()
    );
    let entries = store.audit_entries(owner).unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.event.action == "document.sealed"
                && entry.event.resource.contains(&record.id.to_string()))
            .count(),
        1
    );
}
