use application::vault::{VaultFile, VaultReadLimits};
use application::ApplicationError;
use domain::crypto::{SealedPayload, WrappedDek};

fn vault(plaintext_bytes: usize) -> Vec<u8> {
    VaultFile {
        wrapped_dek: WrappedDek::from_bytes(vec![0; 60]).unwrap(),
        sealed_document: SealedPayload::from_bytes(vec![0; plaintext_bytes + 28]).unwrap(),
    }
    .to_bytes()
    .unwrap()
}

#[test]
fn aes_layout_derives_the_empty_and_maximum_document_sizes() {
    let limits = VaultReadLimits::aes256_gcm(16 * 1024 * 1024).unwrap();
    assert_eq!(limits.max_plaintext_bytes(), 16_777_216);
    assert_eq!(limits.max_vault_bytes(), 16_777_313);
    assert_eq!(limits.inspect(&vault(0)).unwrap(), 0);
    assert_eq!(limits.inspect(&vault(16_777_216)).unwrap(), 16_777_216);
    assert!(matches!(
        limits.inspect(&vault(16_777_217)),
        Err(ApplicationError::StageSupportTooLarge)
    ));
}

#[test]
fn an_empty_only_policy_accepts_the_complete_empty_envelope() {
    let limits = VaultReadLimits::aes256_gcm(0).unwrap();
    assert_eq!(limits.max_vault_bytes(), 97);
    assert_eq!(limits.inspect(&vault(0)).unwrap(), 0);
    assert!(limits.inspect(&vault(1)).is_err());
}

#[test]
fn configured_sizes_cannot_overflow_the_serialized_envelope() {
    for value in [usize::MAX, usize::MAX - 96] {
        assert!(matches!(
            VaultReadLimits::aes256_gcm(value),
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
    assert_eq!(
        VaultReadLimits::aes256_gcm(usize::MAX - 97)
            .unwrap()
            .max_vault_bytes(),
        usize::MAX
    );
}

#[test]
fn malformed_magic_header_and_payload_are_rejected_before_parsing() {
    let limits = VaultReadLimits::aes256_gcm(100).unwrap();
    let mut wrong_magic = vault(1);
    wrong_magic[0] = b'X';
    for bytes in [
        Vec::new(),
        b"DVLT1".to_vec(),
        vault(0)[..96].to_vec(),
        wrong_magic,
    ] {
        assert!(matches!(
            limits.inspect(&bytes),
            Err(ApplicationError::MalformedVaultFile(_))
        ));
    }
}

#[test]
fn the_wrapped_aes_data_key_must_have_exactly_sixty_bytes() {
    let limits = VaultReadLimits::aes256_gcm(100).unwrap();
    for length in [0, 28, 59, 61, u32::MAX] {
        let mut bytes = vault(10);
        bytes[5..9].copy_from_slice(&length.to_be_bytes());
        assert!(matches!(
            limits.inspect(&bytes),
            Err(ApplicationError::MalformedVaultFile(_))
        ));
    }
}

#[test]
fn storage_can_validate_a_header_and_scalar_size_without_reading_the_blob() {
    let limits = VaultReadLimits::aes256_gcm(100).unwrap();
    let bytes = vault(10);
    assert_eq!(limits.inspect_header(&bytes[..9], bytes.len()).unwrap(), 10);
    assert!(limits.inspect_header(&bytes[..8], bytes.len()).is_err());
    assert!(matches!(
        limits.inspect_header(&bytes[..9], 198),
        Err(ApplicationError::StageSupportTooLarge)
    ));
}

#[test]
fn restricted_support_admission_does_not_change_generic_vault_parsing() {
    let mut bytes = vault(10);
    bytes[5..9].copy_from_slice(&59u32.to_be_bytes());
    assert!(VaultFile::parse(&bytes).is_ok());
    assert!(VaultReadLimits::aes256_gcm(100)
        .unwrap()
        .inspect(&bytes)
        .is_err());
}
