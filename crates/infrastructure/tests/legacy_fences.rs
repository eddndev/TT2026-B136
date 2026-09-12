#[allow(dead_code)]
mod legacy_database_support;

use application::documents::DocumentRepository;
use domain::audit::AuditLog;
use domain::clock::OffsetDateTime;
use infrastructure::{FileAuditLog, FileDocumentRepository, RingSha256Hasher};
use legacy_database_support::{Database, Source};
use std::fs;

fn writers_are_fenced(source: &Source) {
    let repository = FileDocumentRepository::new(source.dir.path().join("documents")).unwrap();
    let mut record = repository.find(source.id).unwrap().unwrap();
    record.id = domain::crypto::DocumentId::new();
    assert!(repository.insert(record).is_err());
    let mut log = FileAuditLog::new(source.audit_path(), RingSha256Hasher::new());
    assert!(log
        .append(
            "writer",
            "must_not_append",
            "resource",
            OffsetDateTime::now_utc()
        )
        .is_err());
}

#[test]
fn failed_database_commit_keeps_a_durable_source_fence_until_retry_completes() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    database.seed_case(source.case_id);
    database
        .client
        .batch_execute(
            "CREATE FUNCTION reject_import() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN RAISE EXCEPTION 'injected failure'; END $$;
        CREATE TRIGGER reject_import BEFORE INSERT ON migration_receipts
        FOR EACH ROW EXECUTE FUNCTION reject_import();",
        )
        .unwrap();
    let import = source.inspect();
    assert!(import.apply(&database.url).is_err());
    assert_eq!(database.counts(), (0, 0, 0));
    assert!(source
        .dir
        .path()
        .join("documents/.migration-pending")
        .is_file());
    assert!(source
        .dir
        .path()
        .join("audit.jsonl.migration-pending")
        .is_file());
    writers_are_fenced(&source);
    database
        .client
        .batch_execute("DROP TRIGGER reject_import ON migration_receipts")
        .unwrap();
    source.inspect().apply(&database.url).unwrap();
    assert_eq!(database.counts(), (1, 3, 1));
    assert!(!source
        .dir
        .path()
        .join("documents/.migration-pending")
        .exists());
    assert!(!source
        .dir
        .path()
        .join("audit.jsonl.migration-pending")
        .exists());
    writers_are_fenced(&source);
}

#[test]
fn a_postcommit_marker_failure_reports_committed_state_and_preserves_source_fences() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    database.seed_case(source.case_id);
    // A directory at the completion marker path injects a filesystem failure.
    fs::create_dir(source.dir.path().join("documents/.migrated")).unwrap();
    let import = source.inspect();
    let error = import.apply(&database.url).unwrap_err();
    assert!(error.to_string().contains("committed"));
    assert_eq!(database.counts(), (1, 3, 1));
    writers_are_fenced(&source);
    fs::remove_dir(source.dir.path().join("documents/.migrated")).unwrap();
    source.inspect().apply(&database.url).unwrap();
    assert_eq!(database.counts(), (1, 3, 1));
    writers_are_fenced(&source);
}
