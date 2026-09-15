#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;

use application::documents::DocumentProcessor;
use domain::crypto::{
    document_aad, AuthenticatedCipher, DocumentId, DocumentVersion, KeyManager, SealedPayload,
    WrappedDek,
};
use domain::DomainError;
use zeroize::Zeroizing;

mockall::mock! {
    Cipher {}
    impl AuthenticatedCipher for Cipher {
        fn seal(&self, key: &[u8], aad: &[u8], plaintext: &[u8]) -> Result<SealedPayload, DomainError>;
        fn open(&self, key: &[u8], aad: &[u8], payload: &SealedPayload) -> Result<Vec<u8>, DomainError>;
    }
}

mockall::mock! {
    Keys {}
    impl KeyManager for Keys {
        fn generate_dek(&self) -> Result<Vec<u8>, DomainError>;
        fn wrap_dek(&self, kek: &[u8], dek: &[u8]) -> Result<WrappedDek, DomainError>;
        fn unwrap_dek(&self, kek: &[u8], wrapped: &WrappedDek) -> Result<Vec<u8>, DomainError>;
        fn rewrap_dek(&self, old_kek: &[u8], new_kek: &[u8], wrapped: &WrappedDek) -> Result<WrappedDek, DomainError>;
    }
}

#[test]
fn preparing_versions_requests_fresh_keys_and_binds_each_explicit_context() {
    let id = DocumentId::new();
    let mut cipher = MockCipher::new();
    let mut keys = MockKeys::new();
    let mut sequence = mockall::Sequence::new();
    for number in [1, 2] {
        keys.expect_generate_dek()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move || Ok(vec![number; 32]));
        keys.expect_wrap_dek()
            .times(1)
            .withf(move |kek, dek| kek == [0x44; 32] && dek == [number; 32])
            .returning(move |_, _| WrappedDek::from_bytes(vec![number; 60]));
        cipher
            .expect_seal()
            .times(1)
            .withf(move |key, aad, bytes| {
                key == [number; 32]
                    && aad == document_aad(id, DocumentVersion::new(u32::from(number)).unwrap())
                    && bytes == b"identical content"
            })
            .returning(move |_, _, _| SealedPayload::from_bytes(vec![number; 45]));
    }
    let mut ports = crypto::processor_ports();
    ports.cipher = Box::new(cipher);
    ports.keys = Box::new(keys);
    let processor = DocumentProcessor::new(
        ports,
        crypto::evidence_material(),
        Zeroizing::new(vec![0x44; 32]),
    )
    .unwrap();
    let first = processor
        .prepare_version(
            id,
            DocumentVersion::initial(),
            "first.txt",
            b"identical content",
        )
        .unwrap();
    let second = processor
        .prepare_version(
            id,
            DocumentVersion::new(2).unwrap(),
            "second.txt",
            b"identical content",
        )
        .unwrap();
    assert_eq!(first.id, second.id);
    assert_ne!(first.vault, second.vault);
    assert_eq!(first.digest, second.digest);
    assert!(!first.is_sealed() && !second.is_sealed());
}

#[test]
fn invalid_new_version_names_are_rejected_before_key_generation() {
    let mut ports = crypto::processor_ports();
    ports.cipher = Box::new(MockCipher::new());
    ports.keys = Box::new(MockKeys::new());
    let processor = DocumentProcessor::new(
        ports,
        crypto::evidence_material(),
        Zeroizing::new(vec![0x44; 32]),
    )
    .unwrap();
    assert!(processor
        .prepare_version(
            DocumentId::new(),
            DocumentVersion::new(2).unwrap(),
            "../unsafe",
            b"data"
        )
        .is_err());
}
