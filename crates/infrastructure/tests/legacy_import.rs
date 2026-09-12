use std::fs;

use application::documents::{DocumentRecord, DocumentRepository};
use application::vault::encrypt_with_ports;
use domain::audit::AuditLog;
use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentHasher, DocumentId, DocumentVersion};
use infrastructure::{
    EnvelopeKeyManager, FileAuditLog, FileDocumentRepository, LegacyImport, RingAesGcmCipher,
    RingSha256Hasher,
};
use uuid::Uuid;

fn fixture() -> (tempfile::TempDir, DocumentId, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let id = DocumentId::new();
    let version = DocumentVersion::initial();
    let bytes = b"unchanged historical document";
    let vault = encrypt_with_ports(
        &RingAesGcmCipher::new(),
        &EnvelopeKeyManager::new(),
        &[3; 32],
        bytes,
        id,
        version,
    )
    .unwrap();
    let record = DocumentRecord::pending(
        id,
        version,
        "document.txt".into(),
        RingSha256Hasher::new().hash_bytes(bytes),
        vault,
    )
    .unwrap();
    FileDocumentRepository::new(dir.path().join("documents"))
        .unwrap()
        .insert(record)
        .unwrap();
    FileAuditLog::new(dir.path().join("audit.jsonl"), RingSha256Hasher::new())
        .append(
            "owner",
            "document.uploaded",
            &format!("document:{id}"),
            OffsetDateTime::from_unix_timestamp_nanos(1_735_689_600_123_456_789).unwrap(),
        )
        .unwrap();
    let map = dir.path().join("mapping.json");
    fs::write(
        &map,
        serde_json::to_vec(&serde_json::json!({"documents":[{
            "document_id": id.to_string(), "case_id": Uuid::new_v4().to_string()
        }]}))
        .unwrap(),
    )
    .unwrap();
    (dir, id, map)
}

#[test]
fn inspection_preserves_source_bytes_and_includes_nanosecond_audit_history() {
    let (dir, id, map) = fixture();
    let record = dir.path().join(format!("documents/{id}.json"));
    let before = fs::read(&record).unwrap();
    let audit_before = fs::read(dir.path().join("audit.jsonl")).unwrap();
    let inspection = LegacyImport::inspect(dir.path(), &map, &[3; 32]).unwrap();
    assert_eq!(inspection.report().documents, 1);
    assert_eq!(inspection.report().audit_entries, 1);
    assert_eq!(before, fs::read(record).unwrap());
    assert_eq!(
        audit_before,
        fs::read(dir.path().join("audit.jsonl")).unwrap()
    );
    assert!(!dir.path().join("documents/.migrated").exists());
}

#[test]
fn inspection_rejects_unmapped_duplicate_or_unknown_documents() {
    let (dir, id, map) = fixture();
    for documents in [
        serde_json::json!([]),
        serde_json::json!([
            {"document_id":id.to_string(),"case_id":Uuid::new_v4().to_string()},
            {"document_id":id.to_string(),"case_id":Uuid::new_v4().to_string()}
        ]),
        serde_json::json!([
            {"document_id":Uuid::new_v4().to_string(),"case_id":Uuid::new_v4().to_string()}
        ]),
    ] {
        fs::write(
            &map,
            serde_json::to_vec(&serde_json::json!({"documents":documents})).unwrap(),
        )
        .unwrap();
        assert!(LegacyImport::inspect(dir.path(), &map, &[3; 32]).is_err());
    }
}

#[test]
fn inspection_rejects_wrong_key_changed_metadata_and_broken_history() {
    let (dir, id, map) = fixture();
    assert!(LegacyImport::inspect(dir.path(), &map, &[4; 32]).is_err());
    let path = dir.path().join(format!("documents/{id}.json"));
    let original = fs::read(&path).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    value["digest"] = serde_json::Value::String("00".repeat(32));
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(LegacyImport::inspect(dir.path(), &map, &[3; 32]).is_err());
    fs::write(path, original).unwrap();
    let audit = dir.path().join("audit.jsonl");
    let changed = fs::read_to_string(&audit)
        .unwrap()
        .replace("document.uploaded", "document.deleted");
    fs::write(audit, changed).unwrap();
    assert!(LegacyImport::inspect(dir.path(), &map, &[3; 32]).is_err());
}
