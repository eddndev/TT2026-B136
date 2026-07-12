//! Use case tests exercising the vault flows against mocked ports.

use application::vault::{DecryptDocument, EncryptDocument, RotateKek, VaultFile};
use application::ApplicationError;
use domain::crypto::cipher::{document_aad, AuthenticatedCipher, SealedPayload};
use domain::crypto::document::{DocumentId, DocumentVersion};
use domain::crypto::keys::{KeyManager, WrappedDek};
use domain::DomainError;

mockall::mock! {
    Cipher {}

    impl AuthenticatedCipher for Cipher {
        fn seal(
            &self,
            key: &[u8],
            aad: &[u8],
            plaintext: &[u8],
        ) -> Result<SealedPayload, DomainError>;

        fn open(
            &self,
            key: &[u8],
            aad: &[u8],
            payload: &SealedPayload,
        ) -> Result<Vec<u8>, DomainError>;
    }
}

mockall::mock! {
    Keys {}

    impl KeyManager for Keys {
        fn generate_dek(&self) -> Result<Vec<u8>, DomainError>;

        fn wrap_dek(&self, kek: &[u8], dek: &[u8]) -> Result<WrappedDek, DomainError>;

        fn unwrap_dek(&self, kek: &[u8], wrapped: &WrappedDek) -> Result<Vec<u8>, DomainError>;

        fn rewrap_dek(
            &self,
            old_kek: &[u8],
            new_kek: &[u8],
            wrapped: &WrappedDek,
        ) -> Result<WrappedDek, DomainError>;
    }
}

const KEK: [u8; 32] = [0x11; 32];
const NEW_KEK: [u8; 32] = [0x22; 32];
const DEK: [u8; 32] = [0x33; 32];

fn doc_id() -> DocumentId {
    DocumentId::from_uuid(uuid_from_byte(0x44))
}

fn uuid_from_byte(byte: u8) -> uuid::Uuid {
    uuid::Uuid::from_bytes([byte; 16])
}

fn version() -> DocumentVersion {
    DocumentVersion::new(3).unwrap()
}

fn wrapped_fixture() -> WrappedDek {
    WrappedDek::from_bytes(vec![0x55; 60]).unwrap()
}

fn sealed_fixture() -> SealedPayload {
    SealedPayload::from_bytes(vec![0x66; 40]).unwrap()
}

#[test]
fn encrypt_seals_with_a_fresh_dek_and_the_document_aad() {
    let mut cipher = MockCipher::new();
    let mut keys = MockKeys::new();

    keys.expect_generate_dek()
        .times(1)
        .returning(|| Ok(DEK.to_vec()));
    keys.expect_wrap_dek()
        .times(1)
        .withf(|kek, dek| kek == KEK && dek == DEK)
        .returning(|_, _| Ok(wrapped_fixture()));
    cipher
        .expect_seal()
        .times(1)
        .withf(|key, aad, plaintext| {
            key == DEK && aad == document_aad(doc_id(), version()) && plaintext == b"the document"
        })
        .returning(|_, _, _| Ok(sealed_fixture()));

    let vault_bytes = EncryptDocument::new(cipher, keys)
        .execute(&KEK, b"the document", doc_id(), version())
        .unwrap();

    let vault = VaultFile::parse(&vault_bytes).unwrap();
    assert_eq!(vault.wrapped_dek, wrapped_fixture());
    assert_eq!(vault.sealed_document, sealed_fixture());
}

#[test]
fn decrypt_unwraps_the_dek_and_opens_with_the_rebuilt_aad() {
    let mut cipher = MockCipher::new();
    let mut keys = MockKeys::new();

    keys.expect_unwrap_dek()
        .times(1)
        .withf(|kek, wrapped| kek == KEK && *wrapped == wrapped_fixture())
        .returning(|_, _| Ok(DEK.to_vec()));
    cipher
        .expect_open()
        .times(1)
        .withf(|key, aad, payload| {
            key == DEK && aad == document_aad(doc_id(), version()) && *payload == sealed_fixture()
        })
        .returning(|_, _, _| Ok(b"the document".to_vec()));

    let vault_bytes = VaultFile {
        wrapped_dek: wrapped_fixture(),
        sealed_document: sealed_fixture(),
    }
    .to_bytes()
    .unwrap();

    let plaintext = DecryptDocument::new(cipher, keys)
        .execute(&KEK, &vault_bytes, doc_id(), version())
        .unwrap();
    assert_eq!(plaintext, b"the document");
}

#[test]
fn decrypt_propagates_the_opaque_authentication_failure() {
    let mut cipher = MockCipher::new();
    let mut keys = MockKeys::new();

    keys.expect_unwrap_dek().returning(|_, _| Ok(DEK.to_vec()));
    cipher
        .expect_open()
        .returning(|_, _, _| Err(DomainError::AuthenticationFailed));

    let vault_bytes = VaultFile {
        wrapped_dek: wrapped_fixture(),
        sealed_document: sealed_fixture(),
    }
    .to_bytes()
    .unwrap();

    let err = DecryptDocument::new(cipher, keys)
        .execute(&KEK, &vault_bytes, doc_id(), version())
        .unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::Domain(DomainError::AuthenticationFailed)
    ));
}

#[test]
fn decrypt_rejects_bytes_that_are_not_a_vault_file() {
    let cipher = MockCipher::new();
    let keys = MockKeys::new();

    let err = DecryptDocument::new(cipher, keys)
        .execute(&KEK, b"not a vault file", doc_id(), version())
        .unwrap_err();
    assert!(matches!(err, ApplicationError::MalformedVaultFile(_)));
}

#[test]
fn rotate_rewraps_the_dek_and_copies_the_sealed_document_through() {
    let mut keys = MockKeys::new();
    let rewrapped = WrappedDek::from_bytes(vec![0x77; 60]).unwrap();
    let expected = rewrapped.clone();

    keys.expect_rewrap_dek()
        .times(1)
        .withf(|old_kek, new_kek, wrapped| {
            old_kek == KEK && new_kek == NEW_KEK && *wrapped == wrapped_fixture()
        })
        .returning(move |_, _, _| Ok(rewrapped.clone()));

    let vault_bytes = VaultFile {
        wrapped_dek: wrapped_fixture(),
        sealed_document: sealed_fixture(),
    }
    .to_bytes()
    .unwrap();

    let rotated_bytes = RotateKek::new(keys)
        .execute(&KEK, &NEW_KEK, &vault_bytes)
        .unwrap();

    let rotated = VaultFile::parse(&rotated_bytes).unwrap();
    assert_eq!(rotated.wrapped_dek, expected);
    assert_eq!(rotated.sealed_document, sealed_fixture());
}

#[test]
fn encrypt_propagates_a_key_generation_failure() {
    let cipher = MockCipher::new();
    let mut keys = MockKeys::new();

    keys.expect_generate_dek()
        .returning(|| Err(DomainError::CryptoBackendFailure("csprng down".to_string())));

    let err = EncryptDocument::new(cipher, keys)
        .execute(&KEK, b"the document", doc_id(), version())
        .unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::Domain(DomainError::CryptoBackendFailure(_))
    ));
}
