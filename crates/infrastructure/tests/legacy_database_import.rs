mod legacy_database_support;

use std::fs;

use application::documents::DocumentRepository;
use application::vault::decrypt_with_ports;
use domain::audit::{verify_chain, AuditLog, ChainVerification};
use domain::clock::OffsetDateTime;
use domain::crypto::DocumentVersion;
use infrastructure::{
    EnvelopeKeyManager, FileAuditLog, FileDocumentRepository, PostgresAuditLog, RingAesGcmCipher,
    RingSha256Hasher,
};
use legacy_database_support::{Database, Source, KEK, PLAINTEXT};

#[test]
fn import_preserves_encrypted_bytes_and_exact_nanosecond_audit_prefix() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    database.seed_case(source.case_id);
    let document_before = fs::read(source.document_path()).unwrap();
    let audit_before = fs::read(source.audit_path()).unwrap();
    let inspection = source.inspect();
    inspection.check_target(&database.url).unwrap();
    assert_eq!(database.counts(), (0, 0, 0));
    source.assert_unmarked();
    assert_eq!(inspection.report().documents, 1);
    assert_eq!(inspection.report().audit_entries, 2);
    assert_eq!(
        inspection.report().audit_head,
        source.entries[1].chain.to_hex()
    );

    inspection.apply(&database.url).unwrap();

    assert_eq!(database.counts(), (1, 3, 1));
    let row = database
        .client
        .query_one(
            "SELECT case_id,vault FROM documents WHERE id=$1",
            &[&source.id.as_uuid()],
        )
        .unwrap();
    assert_eq!(row.get::<_, uuid::Uuid>(0), source.case_id.as_uuid());
    let vault: Vec<u8> = row.get(1);
    assert_eq!(vault, source.vault);
    assert_eq!(
        decrypt_with_ports(
            &RingAesGcmCipher::new(),
            &EnvelopeKeyManager::new(),
            &KEK,
            &vault,
            source.id,
            DocumentVersion::initial()
        )
        .unwrap(),
        PLAINTEXT
    );
    let log = PostgresAuditLog::connect(&database.url).unwrap();
    let entries = log.load_all().unwrap();
    assert_eq!(entries[..2], source.entries);
    assert_eq!(entries[2].event.action, "migration.imported");
    assert_eq!(entries[2].event.resource, inspection.report().fingerprint);
    assert!(matches!(
        verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
        ChainVerification::Valid { .. }
    ));
    let timestamps = database
        .client
        .query(
            "SELECT timestamp FROM audit_events WHERE sequence<2 ORDER BY sequence",
            &[],
        )
        .unwrap();
    for (row, expected) in timestamps.iter().zip(&source.entries) {
        assert_eq!(
            row.get::<_, String>(0),
            expected.event.timestamp_rfc3339().unwrap()
        );
    }
    assert_eq!(fs::read(source.document_path()).unwrap(), document_before);
    assert_eq!(fs::read(source.audit_path()).unwrap(), audit_before);
}

#[test]
fn repeated_import_reconciles_receipt_without_rewriting_or_appending_history() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    database.seed_case(source.case_id);
    source.inspect().apply(&database.url).unwrap();
    let mut audit = PostgresAuditLog::connect(&database.url).unwrap();
    audit
        .append(
            "owner@example.test",
            "identity.logout",
            "session",
            OffsetDateTime::now_utc(),
        )
        .unwrap();
    let before = database.stored_state();
    // A committed database import can be retried after interrupted marker creation.
    fs::remove_file(source.dir.path().join("documents/.migrated")).unwrap();
    fs::remove_file(source.dir.path().join("audit.jsonl.migrated")).unwrap();

    let retry = source.inspect();
    retry.check_target(&database.url).unwrap();
    retry.apply(&database.url).unwrap();

    assert_eq!(database.stored_state(), before);
    assert!(source.dir.path().join("documents/.migrated").is_file());
    assert!(source.dir.path().join("audit.jsonl.migrated").is_file());
}

#[test]
fn unknown_destination_case_rejects_check_and_apply_without_mutations() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    let inspection = source.inspect();

    assert!(inspection.check_target(&database.url).is_err());
    assert!(inspection.apply(&database.url).is_err());

    assert_eq!(database.counts(), (0, 0, 0));
    source.assert_unmarked();
}

#[test]
fn audit_insert_failure_rolls_back_documents_history_and_receipt() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    database.seed_case(source.case_id);
    let inspection = source.inspect();
    database
        .client
        .batch_execute(
            "CREATE FUNCTION reject_import_audit() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN IF NEW.action='migration.imported' THEN
           RAISE EXCEPTION 'injected audit failure'; END IF; RETURN NEW; END; $$;
         CREATE TRIGGER reject_import_audit BEFORE INSERT ON audit_events
         FOR EACH ROW EXECUTE FUNCTION reject_import_audit()",
        )
        .unwrap();

    assert!(inspection.apply(&database.url).is_err());

    assert_eq!(database.counts(), (0, 0, 0));
    source.assert_unmarked();
    database
        .client
        .batch_execute("DROP TRIGGER reject_import_audit ON audit_events")
        .unwrap();
    inspection.apply(&database.url).unwrap();
    assert_eq!(database.counts(), (1, 3, 1));
}

#[test]
fn source_or_mapping_changes_after_inspection_refuse_the_entire_import() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    database.seed_case(source.case_id);
    for path in [
        source.document_path(),
        source.audit_path(),
        source.mapping.clone(),
    ] {
        let inspection = source.inspect();
        let original = fs::read(&path).unwrap();
        let mut changed = original.clone();
        changed.push(b'\n');
        fs::write(&path, changed).unwrap();

        assert!(inspection.apply(&database.url).is_err());

        assert_eq!(database.counts(), (0, 0, 0));
        source.assert_unmarked();
        fs::write(path, original).unwrap();
    }
}

#[test]
fn successful_import_fences_legacy_document_and_audit_writers_but_keeps_reads() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    database.seed_case(source.case_id);
    source.inspect().apply(&database.url).unwrap();
    let repository = FileDocumentRepository::new(source.dir.path().join("documents")).unwrap();
    let record = repository.find(source.id).unwrap().unwrap();
    let mut log = FileAuditLog::new(source.audit_path(), RingSha256Hasher::new());
    let document_before = fs::read(source.document_path()).unwrap();
    let audit_before = fs::read(source.audit_path()).unwrap();

    let fresh_source = Source::new();
    let fresh_record = FileDocumentRepository::new(fresh_source.dir.path().join("documents"))
        .unwrap()
        .find(fresh_source.id)
        .unwrap()
        .unwrap();
    assert!(repository.replace(record).is_err());
    assert!(repository.insert(fresh_record).is_err());
    assert!(repository.find(fresh_source.id).unwrap().is_none());
    assert!(log
        .append(
            "owner@example.test",
            "document.uploaded",
            "document:blocked",
            OffsetDateTime::now_utc()
        )
        .is_err());

    assert_eq!(log.load_all().unwrap(), source.entries);
    assert_eq!(fs::read(source.document_path()).unwrap(), document_before);
    assert_eq!(fs::read(source.audit_path()).unwrap(), audit_before);
    assert_eq!(database.counts(), (1, 3, 1));
}

#[test]
fn retry_rejects_corrupted_post_import_history_without_recreating_markers() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    database.seed_case(source.case_id);
    source.inspect().apply(&database.url).unwrap();
    PostgresAuditLog::connect(&database.url)
        .unwrap()
        .append(
            "owner@example.test",
            "identity.logout",
            "session",
            OffsetDateTime::now_utc(),
        )
        .unwrap();
    database
        .client
        .execute(
            "UPDATE audit_events SET actor='changed@example.test' WHERE sequence=3",
            &[],
        )
        .unwrap();
    fs::remove_file(source.dir.path().join("documents/.migrated")).unwrap();
    fs::remove_file(source.dir.path().join("audit.jsonl.migrated")).unwrap();
    let before = database.stored_state();
    let retry = source.inspect();

    let checked = retry.check_target(&database.url);
    let applied = retry.apply(&database.url);

    assert!(
        checked.is_err(),
        "a receipt must not bless a corrupt audit suffix"
    );
    assert!(
        applied.is_err(),
        "retry must validate the destination audit chain"
    );
    assert_eq!(database.stored_state(), before);
    source.assert_unmarked();
}

#[test]
fn startup_rejects_missing_or_corrupted_import_data_despite_a_matching_receipt() {
    for sql in [
        "DELETE FROM documents",
        "TRUNCATE audit_events",
        "UPDATE audit_events SET actor='tampered' WHERE sequence=0",
        "UPDATE audit_events SET actor='tampered' WHERE action='migration.imported'",
        "ALTER TABLE documents DISABLE TRIGGER documents_preserve_evidence;          UPDATE documents SET case_id=(SELECT id FROM cases WHERE title='Other case')",
    ] {
        let Some(mut database) = Database::new() else { return; };
        let source = Source::new();
        database.seed_case(source.case_id);
        source.inspect().apply(&database.url).unwrap();
        database.client.batch_execute(
            "INSERT INTO cases(id,title,reference,created_by)              SELECT '00000000-0000-4000-8000-000000000001','Other case','OTHER',created_by FROM cases LIMIT 1"
        ).unwrap();
        let runtime_url = database.restricted_url();
        infrastructure::legacy::require_completed_import(source.dir.path(), &runtime_url).unwrap();
        // Simulate an incomplete administrative restore, bypassing normal constraints.
        database.client.batch_execute(&format!("SET session_replication_role=replica; {sql}; SET session_replication_role=origin")).unwrap();

        assert!(infrastructure::legacy::require_completed_import(
            source.dir.path(), &runtime_url
        ).is_err(), "startup accepted inconsistent database after: {sql}");
    }
}

#[test]
fn startup_accepts_later_history_but_rejects_changed_case_assignments_in_markers() {
    let Some(mut database) = Database::new() else {
        return;
    };
    let source = Source::new();
    database.seed_case(source.case_id);
    source.inspect().apply(&database.url).unwrap();
    PostgresAuditLog::connect(&database.url)
        .unwrap()
        .append(
            "owner@example.test",
            "identity.logout",
            "session",
            OffsetDateTime::now_utc(),
        )
        .unwrap();
    let runtime_url = database.restricted_url();
    infrastructure::legacy::require_completed_import(source.dir.path(), &runtime_url).unwrap();
    let marker = source.dir.path().join("documents/.migrated");
    let mut report: serde_json::Value =
        serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    report["assignments"][source.id.to_string()] =
        serde_json::Value::String("00000000-0000-4000-8000-000000000001".into());
    for path in [marker, source.dir.path().join("audit.jsonl.migrated")] {
        fs::write(path, serde_json::to_vec(&report).unwrap()).unwrap();
    }

    assert!(
        infrastructure::legacy::require_completed_import(source.dir.path(), &runtime_url).is_err(),
        "case assignments must remain bound to the import fingerprint"
    );
}
