//! Vault file format and the use cases that encrypt, decrypt, and rotate
//! keys for stored documents.
//!
//! This module is the single definition of the vault file format. A vault
//! file carries one encrypted document together with the wrapped data key
//! that encrypts it:
//!
//! ```text
//! offset  size  field
//! 0       5     magic bytes "DVLT1"
//! 5       4     big-endian byte length N of the wrapped data key
//! 9       N     wrapped data key: nonce || ciphertext || tag, sealed under
//!               the key encryption key with empty AAD
//! 9+N     rest  sealed document: nonce || ciphertext || tag, sealed under
//!               the data key with AAD = document id (16 raw UUID bytes) ||
//!               document version (4 big-endian bytes)
//! ```
//!
//! Rotating the key encryption key replaces only the wrapped data key
//! field; the sealed document bytes are copied through untouched.

use domain::crypto::cipher::{document_aad, AuthenticatedCipher, SealedPayload};
use domain::crypto::document::{DocumentId, DocumentVersion};
use domain::crypto::keys::{KeyManager, WrappedDek};
use zeroize::Zeroizing;

use crate::error::ApplicationError;

mod limits;
pub use limits::VaultReadLimits;

/// Leading magic bytes of every vault file.
pub const VAULT_MAGIC: &[u8; 5] = b"DVLT1";

/// Byte offset where the wrapped data key starts: magic plus length field.
const VAULT_HEADER_LEN: usize = VAULT_MAGIC.len() + 4;

/// In-memory form of a parsed vault file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultFile {
    /// The document's data key, sealed under the key encryption key.
    pub wrapped_dek: WrappedDek,
    /// The document, sealed under the data key.
    pub sealed_document: SealedPayload,
}

impl VaultFile {
    /// Serializes the vault file into the documented byte layout.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ApplicationError> {
        let wrapped = self.wrapped_dek.as_bytes();
        let wrapped_len = u32::try_from(wrapped.len()).map_err(|_| {
            ApplicationError::MalformedVaultFile(
                "wrapped data key does not fit in the 4-byte length field".to_string(),
            )
        })?;
        let sealed = self.sealed_document.as_bytes();
        let mut bytes = Vec::with_capacity(VAULT_HEADER_LEN + wrapped.len() + sealed.len());
        bytes.extend_from_slice(VAULT_MAGIC);
        bytes.extend_from_slice(&wrapped_len.to_be_bytes());
        bytes.extend_from_slice(wrapped);
        bytes.extend_from_slice(sealed);
        Ok(bytes)
    }

    /// Parses the documented byte layout back into its two fields.
    pub fn parse(bytes: &[u8]) -> Result<Self, ApplicationError> {
        let malformed = |detail: &str| ApplicationError::MalformedVaultFile(detail.to_string());
        if bytes.len() < VAULT_HEADER_LEN {
            return Err(malformed("shorter than the fixed header"));
        }
        if &bytes[..VAULT_MAGIC.len()] != VAULT_MAGIC {
            return Err(malformed("missing the DVLT1 magic bytes"));
        }
        let length_field: [u8; 4] = bytes[VAULT_MAGIC.len()..VAULT_HEADER_LEN]
            .try_into()
            .expect("slice is exactly four bytes");
        let wrapped_len = u32::from_be_bytes(length_field) as usize;
        let sealed_start = VAULT_HEADER_LEN
            .checked_add(wrapped_len)
            .ok_or_else(|| malformed("wrapped data key length overflows"))?;
        if bytes.len() < sealed_start {
            return Err(malformed("wrapped data key length exceeds the file size"));
        }
        let wrapped_dek = WrappedDek::from_bytes(bytes[VAULT_HEADER_LEN..sealed_start].to_vec())
            .map_err(|err| malformed(&format!("wrapped data key: {err}")))?;
        let sealed_document = SealedPayload::from_bytes(bytes[sealed_start..].to_vec())
            .map_err(|err| malformed(&format!("sealed document: {err}")))?;
        Ok(Self {
            wrapped_dek,
            sealed_document,
        })
    }
}

/// Encrypts a document into a vault file.
///
/// A fresh data key is generated per document, the document is sealed under
/// it bound to its identity and version, and the data key is stored wrapped
/// under the key encryption key.
pub struct EncryptDocument<C, K> {
    cipher: C,
    keys: K,
}

impl<C: AuthenticatedCipher, K: KeyManager> EncryptDocument<C, K> {
    pub fn new(cipher: C, keys: K) -> Self {
        Self { cipher, keys }
    }

    /// Returns the serialized vault file for `plaintext`.
    pub fn execute(
        &self,
        kek: &[u8],
        plaintext: &[u8],
        id: DocumentId,
        version: DocumentVersion,
    ) -> Result<Vec<u8>, ApplicationError> {
        encrypt_with_ports(&self.cipher, &self.keys, kek, plaintext, id, version)
    }
}

/// Decrypts a vault file back into the document plaintext.
pub struct DecryptDocument<C, K> {
    cipher: C,
    keys: K,
}

impl<C: AuthenticatedCipher, K: KeyManager> DecryptDocument<C, K> {
    pub fn new(cipher: C, keys: K) -> Self {
        Self { cipher, keys }
    }

    /// Unwraps the data key and opens the sealed document, rebuilding the
    /// AAD from the identity and version the caller claims. A wrong id or
    /// version makes the open fail exactly like a tampered ciphertext.
    pub fn execute(
        &self,
        kek: &[u8],
        vault_bytes: &[u8],
        id: DocumentId,
        version: DocumentVersion,
    ) -> Result<Vec<u8>, ApplicationError> {
        decrypt_with_ports(&self.cipher, &self.keys, kek, vault_bytes, id, version)
    }
}

/// Encrypts one document through borrowed cipher and key-manager ports.
pub fn encrypt_with_ports<C, K>(
    cipher: &C,
    keys: &K,
    kek: &[u8],
    plaintext: &[u8],
    id: DocumentId,
    version: DocumentVersion,
) -> Result<Vec<u8>, ApplicationError>
where
    C: AuthenticatedCipher + ?Sized,
    K: KeyManager + ?Sized,
{
    let dek = Zeroizing::new(keys.generate_dek()?);
    let wrapped_dek = keys.wrap_dek(kek, &dek)?;
    let aad = document_aad(id, version);
    let sealed_document = cipher.seal(&dek, &aad, plaintext)?;
    VaultFile {
        wrapped_dek,
        sealed_document,
    }
    .to_bytes()
}

/// Decrypts one vault through borrowed cipher and key-manager ports.
pub fn decrypt_with_ports<C, K>(
    cipher: &C,
    keys: &K,
    kek: &[u8],
    vault_bytes: &[u8],
    id: DocumentId,
    version: DocumentVersion,
) -> Result<Vec<u8>, ApplicationError>
where
    C: AuthenticatedCipher + ?Sized,
    K: KeyManager + ?Sized,
{
    let vault = VaultFile::parse(vault_bytes)?;
    let dek = Zeroizing::new(keys.unwrap_dek(kek, &vault.wrapped_dek)?);
    let aad = document_aad(id, version);
    Ok(cipher.open(&dek, &aad, &vault.sealed_document)?)
}

/// Rotates the key encryption key of a vault file.
///
/// Only the wrapped data key is replaced; the sealed document is copied
/// through byte for byte, so rotation cost is independent of document size.
pub struct RotateKek<K> {
    keys: K,
}

impl<K: KeyManager> RotateKek<K> {
    pub fn new(keys: K) -> Self {
        Self { keys }
    }

    /// Returns the vault file rewrapped from `old_kek` to `new_kek`.
    pub fn execute(
        &self,
        old_kek: &[u8],
        new_kek: &[u8],
        vault_bytes: &[u8],
    ) -> Result<Vec<u8>, ApplicationError> {
        let vault = VaultFile::parse(vault_bytes)?;
        let wrapped_dek = self.keys.rewrap_dek(old_kek, new_kek, &vault.wrapped_dek)?;
        VaultFile {
            wrapped_dek,
            sealed_document: vault.sealed_document,
        }
        .to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use domain::crypto::cipher::SEALED_PAYLOAD_MIN_LEN;

    use super::*;

    fn sample_vault() -> VaultFile {
        VaultFile {
            wrapped_dek: WrappedDek::from_bytes(vec![0xaa; 60]).unwrap(),
            sealed_document: SealedPayload::from_bytes(vec![0xbb; 45]).unwrap(),
        }
    }

    fn assert_malformed(result: Result<VaultFile, ApplicationError>, expected_detail: &str) {
        match result {
            Err(ApplicationError::MalformedVaultFile(detail)) => {
                assert!(
                    detail.contains(expected_detail),
                    "detail {detail:?} should mention {expected_detail:?}"
                );
            }
            other => panic!("expected a malformed vault file error, got {other:?}"),
        }
    }

    #[test]
    fn serialization_round_trips() {
        let vault = sample_vault();
        let bytes = vault.to_bytes().unwrap();
        assert_eq!(VaultFile::parse(&bytes).unwrap(), vault);
    }

    #[test]
    fn serialized_layout_starts_with_magic_and_length() {
        let bytes = sample_vault().to_bytes().unwrap();
        assert_eq!(&bytes[..5], VAULT_MAGIC);
        assert_eq!(&bytes[5..9], &60u32.to_be_bytes());
        assert_eq!(&bytes[9..69], &[0xaa; 60][..]);
        assert_eq!(&bytes[69..], &[0xbb; 45][..]);
    }

    #[test]
    fn parse_rejects_a_truncated_header() {
        assert_malformed(VaultFile::parse(b"DVLT1\x00\x00"), "header");
    }

    #[test]
    fn parse_rejects_wrong_magic_bytes() {
        let mut bytes = sample_vault().to_bytes().unwrap();
        bytes[0] = b'X';
        assert_malformed(VaultFile::parse(&bytes), "magic");
    }

    #[test]
    fn parse_rejects_a_wrapped_key_length_beyond_the_file() {
        let mut bytes = sample_vault().to_bytes().unwrap();
        bytes[5..9].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_malformed(VaultFile::parse(&bytes), "exceeds the file size");
    }

    #[test]
    fn parse_rejects_a_wrapped_key_shorter_than_a_sealed_payload() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(VAULT_MAGIC);
        bytes.extend_from_slice(&4u32.to_be_bytes());
        bytes.extend_from_slice(&[0u8; 4]);
        bytes.extend_from_slice(&[0xbb; 45]);
        assert_malformed(VaultFile::parse(&bytes), "wrapped data key");
    }

    #[test]
    fn parse_rejects_a_sealed_document_shorter_than_nonce_and_tag() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(VAULT_MAGIC);
        bytes.extend_from_slice(&60u32.to_be_bytes());
        bytes.extend_from_slice(&[0xaa; 60]);
        bytes.extend_from_slice(&[0xbb; SEALED_PAYLOAD_MIN_LEN - 1]);
        assert_malformed(VaultFile::parse(&bytes), "sealed document");
    }
}
