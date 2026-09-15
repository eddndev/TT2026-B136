#[allow(dead_code)]
mod metadata_database_support;

use application::documents::{CaseDocumentStore, MetadataRevision};
use application::ApplicationError;
use metadata_database_support::{document, metadata, Fixture};
use postgres::{Client, NoTls};
use std::sync::{mpsc, Arc, Barrier};
use std::time::{Duration, Instant};

#[test]
fn competing_classifications_have_one_winner_while_content_append_remains_independent() {
    let Some(f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    store.insert(f.owner, f.case, record.clone(), f.at).unwrap();
    let mut next = record.clone();
    next.version = record.version.next().unwrap();
    next.vault.push(1);
    let barrier = Arc::new(Barrier::new(5));
    let mut writers = Vec::new();
    for index in 0..3 {
        let writer = f.store();
        let barrier = barrier.clone();
        let (owner, case, id, at) = (f.owner, f.case, record.id, f.at);
        writers.push(std::thread::spawn(move || {
            barrier.wait();
            writer.replace_metadata(
                owner,
                case,
                id,
                MetadataRevision::unclassified(),
                metadata("Type", &format!("C{index}"), &[]),
                at,
            )
        }));
    }
    let appender = f.store();
    let gate = barrier.clone();
    let (owner, case, at) = (f.owner, f.case, f.at);
    let appended = std::thread::spawn(move || {
        gate.wait();
        appender.append(owner, case, record.version, next, at)
    });
    barrier.wait();
    let results = writers
        .into_iter()
        .map(|w| w.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(ApplicationError::DocumentMetadataConflict)))
            .count(),
        2
    );
    let appended = appended.join().unwrap().unwrap();
    let events = store.audit_entries(f.owner).unwrap();
    let changed = events
        .iter()
        .position(|e| e.event.action == "document.metadata_changed")
        .unwrap();
    let content = events
        .iter()
        .position(|e| e.event.action == "document.version_added")
        .unwrap();
    assert_eq!(
        appended.current_metadata.metadata_revision.get(),
        u32::from(changed < content)
    );
    assert_eq!(appended.content.document.version.get(), 2);
    assert_eq!(
        store
            .get_metadata(f.owner, f.case, record.id, f.at)
            .unwrap()
            .metadata_revision
            .get(),
        1
    );
}

#[test]
fn direct_sql_insert_observes_predecessor_commit_after_waiting_for_sequence_lock() {
    for commit in [true, false] {
        let Some(mut f) = Fixture::new() else { return };
        let store = f.store();
        let record = document();
        store.insert(f.owner, f.case, record.clone(), f.at).unwrap();
        let mut first = Client::connect(&f.runtime_url, NoTls).unwrap();
        let mut transaction = first.transaction().unwrap();
        insert(&mut transaction, record.id.as_uuid(), f.owner.as_uuid(), 1).unwrap();
        let (ready, observed) = mpsc::channel();
        let (url, id, actor) = (
            f.runtime_url.clone(),
            record.id.as_uuid(),
            f.owner.as_uuid(),
        );
        let writer = std::thread::spawn(move || {
            let mut client = Client::connect(&url, NoTls).unwrap();
            ready
                .send(
                    client
                        .query_one("SELECT pg_backend_pid()", &[])
                        .unwrap()
                        .get::<_, i32>(0),
                )
                .unwrap();
            insert(&mut client, id, actor, 2)
        });
        wait_for_lock(
            &mut f.db.control,
            observed.recv_timeout(Duration::from_secs(5)).unwrap(),
        );
        if commit {
            transaction.commit().unwrap()
        } else {
            transaction.rollback().unwrap()
        }
        assert_eq!(writer.join().unwrap().is_ok(), commit);
        assert_eq!(
            f.db.client
                .query_one("SELECT COUNT(*) FROM document_metadata_revisions", &[])
                .unwrap()
                .get::<_, i64>(0),
            if commit { 2 } else { 0 }
        );
    }
}

#[test]
fn revocation_committed_while_classification_waits_prevents_mutation_and_audit() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    store.insert(f.owner, f.case, record.clone(), f.at).unwrap();
    let before = f.snapshot();
    let mut admin = Client::connect(&f.db.url, NoTls).unwrap();
    let mut transaction = admin.transaction().unwrap();
    transaction
        .query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])
        .unwrap();
    transaction
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&f.owner.as_uuid()],
        )
        .unwrap();
    let (owner, case, id, at) = (f.owner, f.case, record.id, f.at);
    let (ready, observed) = mpsc::channel();
    let role = f.role.clone();
    let writer = std::thread::spawn(move || {
        ready.send(()).unwrap();
        store.replace_metadata(
            owner,
            case,
            id,
            MetadataRevision::unclassified(),
            metadata("Type", "Class", &[]),
            at,
        )
    });
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if f.db.control.query_one("SELECT EXISTS(SELECT 1 FROM pg_locks l JOIN pg_stat_activity a ON a.pid=l.pid WHERE a.usename=$1 AND l.locktype='advisory' AND NOT l.granted)",&[&role]).unwrap().get::<_,bool>(0) {break;}
        assert!(
            Instant::now() < deadline,
            "classification never waited for authorization lock"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    transaction.commit().unwrap();
    assert!(matches!(
        writer.join().unwrap(),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(f.snapshot(), before);
}

fn insert<C: postgres::GenericClient>(
    client: &mut C,
    id: uuid::Uuid,
    actor: uuid::Uuid,
    revision: i64,
) -> Result<u64, postgres::Error> {
    client.execute("INSERT INTO document_metadata_revisions(document_id,metadata_revision,tags,metadata_digest,changed_at,changed_by,changed_by_email) VALUES($1,$2,ARRAY[]::text[],pg_catalog.sha256(document_metadata_bytes(NULL,NULL,ARRAY[]::text[])),'2025-01-01T00:00:00Z',$3,'author@example.test')",&[&id,&revision,&actor])
}

fn wait_for_lock(control: &mut Client, pid: i32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if control.query_one("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND NOT granted)",&[&pid]).unwrap().get::<_,bool>(0) {return;}
        assert!(
            Instant::now() < deadline,
            "insert never waited for sequence lock"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
