use application::documents::{DocumentRecord, SealedEvidence};
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};

fn evidence() -> SealedEvidence {
    SealedEvidence {
        signature: vec![1, 2, 3],
        timestamp_token: vec![4, 5, 6],
        signer_certificate_pem: b"signer".to_vec(),
        issuer_certificate_pem: b"issuer".to_vec(),
        crl_pem: b"crl".to_vec(),
        tsa_chain_pem: Some(b"tsa".to_vec()),
        openssl_version: "OpenSSL test".to_string(),
    }
}

#[test]
fn a_pending_record_becomes_sealed_once() {
    let id = DocumentId::new();
    let mut record = DocumentRecord::pending(
        id,
        DocumentVersion::initial(),
        "acta.txt".to_string(),
        Sha256Digest::from_array([7; 32]),
        vec![9; 60],
    )
    .unwrap();

    assert!(!record.is_sealed());
    record.seal(evidence()).unwrap();
    assert!(record.is_sealed());
    assert!(record.seal(evidence()).is_err());
}

#[test]
fn a_record_rejects_a_name_that_cannot_enter_an_evidence_archive() {
    let result = DocumentRecord::pending(
        DocumentId::new(),
        DocumentVersion::initial(),
        "../escape.txt".to_string(),
        Sha256Digest::from_array([7; 32]),
        vec![9; 60],
    );

    assert!(result.is_err());
}
