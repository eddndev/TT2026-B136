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

#[test]
fn a_stale_writer_cannot_replace_captured_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let first = FileDocumentRepository::new(dir.path()).unwrap();
    let second = FileDocumentRepository::new(dir.path()).unwrap();
    let mut original = pending_record();
    first.insert(original.clone()).unwrap();
    let mut stale = second.find(original.id).unwrap().unwrap();
    original.seal(evidence()).unwrap();
    first.replace(original.clone()).unwrap();
    let mut other_evidence = evidence();
    other_evidence.timestamp_token.push(7);
    stale.seal(other_evidence).unwrap();

    assert!(matches!(
        second.replace(stale),
        Err(ApplicationError::DocumentAlreadySealed(_))
    ));
    assert_eq!(first.find(original.id).unwrap(), Some(original));
}

#[test]
fn concurrent_sealers_preserve_exactly_one_evidence_record() {
    let dir = tempfile::tempdir().unwrap();
    let repository = FileDocumentRepository::new(dir.path()).unwrap();
    let original = pending_record();
    repository.insert(original.clone()).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|index| {
            let repository = FileDocumentRepository::new(dir.path()).unwrap();
            let mut candidate = original.clone();
            let mut evidence = evidence();
            evidence.timestamp_token.push(index);
            candidate.seal(evidence).unwrap();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                (repository.replace(candidate.clone()), candidate)
            })
        })
        .collect();
    let mut winner = None;
    for worker in workers {
        let (result, candidate) = worker.join().unwrap();
        match result {
            Ok(()) => assert!(winner.replace(candidate).is_none()),
            Err(ApplicationError::DocumentAlreadySealed(_)) => {}
            other => panic!("unexpected replacement result: {other:?}"),
        }
    }
    assert!(winner.is_some());
    assert_eq!(repository.find(original.id).unwrap(), winner);
}

#[test]
fn sealing_cannot_change_encrypted_document_identity_or_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let repository = FileDocumentRepository::new(dir.path()).unwrap();
    let original = pending_record();
    repository.insert(original.clone()).unwrap();
    let mut changes = vec![original.clone(); 4];
    changes[0].name = "different.txt".into();
    changes[1].digest = Sha256Digest::from_array([0xcd; 32]);
    changes[2].vault.push(1);
    changes[3].version = DocumentVersion::new(2).unwrap();
    for mut changed in changes {
        changed.seal(evidence()).unwrap();
        assert!(matches!(
            repository.replace(changed),
            Err(ApplicationError::StoredDocumentInconsistent(_))
        ));
        assert_eq!(
            repository.find(original.id).unwrap(),
            Some(original.clone())
        );
    }
}
