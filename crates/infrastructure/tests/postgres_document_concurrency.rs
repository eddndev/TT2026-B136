mod document_store_support;

use std::sync::{mpsc, Arc, Barrier};
use std::time::{Duration, Instant};

use application::cases::{CaseRecord, CaseRepository};
use application::documents::{CaseDocumentStore, DocumentAction};
use domain::audit::{verify_chain, AuditLog, ChainVerification};
use domain::cases::CaseId;
use domain::identity::Role;
use infrastructure::{
    PostgresAuditLog, PostgresCaseDocumentStore, PostgresCaseRepository, RingSha256Hasher,
};
use postgres::{Client, NoTls};
use time::OffsetDateTime;

use document_store_support::{case, database_url, document, evidence, user};

// Fixtures install temporary constraints or triggers on shared database tables.
static DATABASE_FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn revocation_waits_for_an_authorized_commit_and_blocks_the_next_commit() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let actor = user(&url, Role::Litigator);
    let case_id = case(&url, actor);
    let original = document();
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    store
        .insert(actor, case_id, original.clone(), OffsetDateTime::now_utc())
        .unwrap();
    let mut sealed = original.clone();
    sealed.seal(evidence()).unwrap();
    let seal_adapter = PostgresCaseDocumentStore::connect(&url).unwrap();
    let revoke_adapter = PostgresCaseRepository::connect(&url).unwrap();
    let mut admin = Client::connect(&url, NoTls).unwrap();
    let suffix = original.id.as_uuid().simple().to_string();
    let hold_key = i64::from(
        u32::from_be_bytes(original.id.as_uuid().as_bytes()[..4].try_into().unwrap()) & 0x7fff_ffff,
    );
    admin
        .batch_execute(&format!(
            "CREATE FUNCTION pause_document_{suffix}() RETURNS TRIGGER LANGUAGE plpgsql AS $$
         BEGIN PERFORM pg_advisory_xact_lock({hold_key}); RETURN NEW; END; $$;
         CREATE TRIGGER pause_document_{suffix} BEFORE UPDATE ON documents FOR EACH ROW
         WHEN (NEW.id='{}') EXECUTE FUNCTION pause_document_{suffix}();",
            original.id
        ))
        .unwrap();
    let mut blocker = Client::connect(&url, NoTls).unwrap();
    let mut blocked_transaction = blocker.transaction().unwrap();
    blocked_transaction
        .query_one("SELECT pg_advisory_xact_lock($1)", &[&hold_key])
        .unwrap();
    let expected = sealed.clone();
    let sealer = std::thread::spawn(move || {
        seal_adapter.seal(actor, case_id, sealed, OffsetDateTime::now_utc())
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let blocked: bool = admin.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND classid=0 AND objid::bigint=$1 AND NOT granted)",
            &[&hold_key],
        ).unwrap().get(0);
        if blocked {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "seal never reached its guarded write"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let (done, observed) = mpsc::channel();
    let revoker = std::thread::spawn(move || {
        let result = revoke_adapter.remove_member(case_id, actor, owner);
        done.send(result).unwrap();
    });
    let premature = observed.recv_timeout(Duration::from_millis(100));
    blocked_transaction.commit().unwrap();
    sealer.join().unwrap().unwrap();
    revoker.join().unwrap();
    assert!(matches!(premature, Err(mpsc::RecvTimeoutError::Timeout)));
    observed
        .recv_timeout(Duration::from_secs(1))
        .unwrap()
        .unwrap();
    admin.batch_execute(&format!("DROP TRIGGER pause_document_{suffix} ON documents; DROP FUNCTION pause_document_{suffix}();")).unwrap();
    assert_eq!(
        store
            .load(
                owner,
                case_id,
                original.id,
                application::documents::VersionSelection::Only,
                DocumentAction::Verify
            )
            .unwrap(),
        expected
    );
    assert!(store
        .record_access(
            actor,
            case_id,
            &expected,
            DocumentAction::Export,
            OffsetDateTime::now_utc()
        )
        .is_err());
    let entries = store.audit_entries(owner).unwrap();
    let seal_event = entries
        .iter()
        .find(|entry| {
            entry.event.action == "document.sealed"
                && entry.event.resource.contains(&original.id.to_string())
        })
        .unwrap();
    let revocation = entries
        .iter()
        .find(|entry| {
            entry.event.action == "case.member_removed"
                && entry.event.resource.contains(&case_id.to_string())
        })
        .unwrap();
    assert!(seal_event.event.sequence < revocation.event.sequence);
}

#[test]
fn independent_document_case_and_identity_audit_writers_extend_one_chain() {
    let _guard = DATABASE_FIXTURES.lock().unwrap();
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case_id = case(&url, owner);
    let docs = PostgresCaseDocumentStore::connect(&url).unwrap();
    let cases = PostgresCaseRepository::connect(&url).unwrap();
    let mut audit = PostgresAuditLog::connect(&url).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let doc_barrier = barrier.clone();
    let doc_writer = std::thread::spawn(move || {
        doc_barrier.wait();
        for _ in 0..4 {
            docs.insert(owner, case_id, document(), OffsetDateTime::now_utc())
                .unwrap();
        }
    });
    let case_barrier = barrier.clone();
    let case_writer = std::thread::spawn(move || {
        case_barrier.wait();
        for _ in 0..4 {
            cases
                .insert(CaseRecord {
                    id: CaseId::new(),
                    title: "Concurrent case".into(),
                    reference: "File".into(),
                    created_by: owner,
                })
                .unwrap();
        }
    });
    let audit_writer = std::thread::spawn(move || {
        barrier.wait();
        for _ in 0..4 {
            audit
                .append(
                    "system",
                    "identity.test_event",
                    "identity",
                    OffsetDateTime::now_utc(),
                )
                .unwrap();
        }
    });
    doc_writer.join().unwrap();
    case_writer.join().unwrap();
    audit_writer.join().unwrap();
    let entries = PostgresCaseDocumentStore::connect(&url)
        .unwrap()
        .audit_entries(owner)
        .unwrap();
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.event.sequence, index as u64);
    }
    assert!(matches!(
        verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
        ChainVerification::Valid { .. }
    ));
}
