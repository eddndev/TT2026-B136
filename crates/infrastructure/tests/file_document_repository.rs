use application::documents::{DocumentRecord, DocumentRepository, SealedEvidence};
use application::ApplicationError;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use infrastructure::FileDocumentRepository;

fn pending_record() -> DocumentRecord {
    DocumentRecord::pending(
        DocumentId::new(),
        DocumentVersion::initial(),
        "acta.txt".to_string(),
        Sha256Digest::from_array([0xab; 32]),
        vec![0x42; 80],
    )
    .unwrap()
}

fn evidence() -> SealedEvidence {
    SealedEvidence {
        signature: vec![1, 2, 3],
        timestamp_token: vec![4, 5, 6],
        signer_certificate_pem: b"signer pem".to_vec(),
        issuer_certificate_pem: b"issuer pem".to_vec(),
        crl_pem: b"crl pem".to_vec(),
        tsa_chain_pem: Some(b"tsa chain".to_vec()),
        openssl_version: "OpenSSL test".to_string(),
    }
}

#[test]
fn insert_and_find_round_trip_a_pending_record() {
    let dir = tempfile::tempdir().unwrap();
    let repository = FileDocumentRepository::new(dir.path()).unwrap();
    let record = pending_record();

    repository.insert(record.clone()).unwrap();

    assert_eq!(repository.find(record.id).unwrap(), Some(record));
}

#[test]
fn duplicate_insert_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let repository = FileDocumentRepository::new(dir.path()).unwrap();
    let record = pending_record();
    repository.insert(record.clone()).unwrap();

    let error = repository.insert(record.clone()).unwrap_err();

    assert!(matches!(error, ApplicationError::DocumentAlreadyExists(_)));
}

#[test]
fn replace_persists_sealed_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let repository = FileDocumentRepository::new(dir.path()).unwrap();
    let mut record = pending_record();
    repository.insert(record.clone()).unwrap();
    record.seal(evidence()).unwrap();

    repository.replace(record.clone()).unwrap();

    assert_eq!(repository.find(record.id).unwrap(), Some(record));
}

#[test]
fn replace_rejects_a_missing_record() {
    let dir = tempfile::tempdir().unwrap();
    let repository = FileDocumentRepository::new(dir.path()).unwrap();
    let record = pending_record();

    let error = repository.replace(record).unwrap_err();

    assert!(matches!(error, ApplicationError::DocumentNotFound(_)));
}

#[test]
fn malformed_storage_is_reported_as_a_port_failure() {
    let dir = tempfile::tempdir().unwrap();
    let repository = FileDocumentRepository::new(dir.path()).unwrap();
    let id = DocumentId::new();
    std::fs::write(dir.path().join(format!("{id}.json")), b"not json").unwrap();

    let error = repository.find(id).unwrap_err();

    assert!(matches!(error, ApplicationError::Port(_)));
}

#[test]
fn stored_identity_must_match_the_requested_path() {
    let dir = tempfile::tempdir().unwrap();
    let repository = FileDocumentRepository::new(dir.path()).unwrap();
    let record = pending_record();
    let requested_id = DocumentId::new();
    repository.insert(record.clone()).unwrap();
    std::fs::rename(
        dir.path().join(format!("{}.json", record.id)),
        dir.path().join(format!("{requested_id}.json")),
    )
    .unwrap();

    let error = repository.find(requested_id).unwrap_err();

    assert!(matches!(
        error,
        ApplicationError::StoredDocumentInconsistent(_)
    ));
}
