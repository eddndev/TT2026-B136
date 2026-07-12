//! Authenticated encryption port and the sealed payload it exchanges.

use std::fmt;

use crate::crypto::document::{DocumentId, DocumentVersion};
use crate::error::DomainError;

/// Key length in bytes for AES-256-GCM.
pub const AES_256_GCM_KEY_LEN: usize = 32;

/// Nonce length in bytes: AES-GCM with a 96-bit nonce.
pub const AES_GCM_NONCE_LEN: usize = 12;

/// Authentication tag length in bytes: a full 128-bit GCM tag.
pub const AES_GCM_TAG_LEN: usize = 16;

/// Smallest valid sealed payload: a nonce and a tag around an empty
/// ciphertext.
pub const SEALED_PAYLOAD_MIN_LEN: usize = AES_GCM_NONCE_LEN + AES_GCM_TAG_LEN;

/// Byte length of the additional authenticated data built by
/// [`document_aad`]: 16 bytes of document id plus 4 bytes of version.
pub const DOCUMENT_AAD_LEN: usize = 20;

/// A sealed (encrypted and authenticated) byte sequence.
///
/// Every sealing operation in this system, for documents and for wrapped
/// data keys alike, produces bytes with this fixed layout:
///
/// ```text
/// nonce (12 bytes) || ciphertext (same length as the plaintext) || tag (16 bytes)
/// ```
///
/// The nonce is drawn fresh from a cryptographically secure random source
/// for every seal operation and stored in clear; it needs uniqueness, not
/// secrecy. The tag authenticates the ciphertext together with the
/// additional authenticated data supplied at sealing time.
#[derive(Clone, PartialEq, Eq)]
pub struct SealedPayload(Vec<u8>);

impl SealedPayload {
    /// Validates and wraps bytes in the `nonce || ciphertext || tag` layout.
    ///
    /// Rejects anything shorter than [`SEALED_PAYLOAD_MIN_LEN`]; the content
    /// itself is only verified when an adapter opens the payload.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, DomainError> {
        if bytes.len() < SEALED_PAYLOAD_MIN_LEN {
            return Err(DomainError::MalformedSealedPayload {
                min: SEALED_PAYLOAD_MIN_LEN,
                actual: bytes.len(),
            });
        }
        Ok(Self(bytes))
    }

    /// Borrows the full `nonce || ciphertext || tag` byte sequence.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consumes the payload, returning the full byte sequence.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Borrows the leading 12-byte nonce.
    pub fn nonce(&self) -> &[u8] {
        &self.0[..AES_GCM_NONCE_LEN]
    }

    /// Borrows the ciphertext followed by the 16-byte tag.
    pub fn ciphertext_and_tag(&self) -> &[u8] {
        &self.0[AES_GCM_NONCE_LEN..]
    }
}

impl fmt::Debug for SealedPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SealedPayload({} bytes)", self.0.len())
    }
}

/// Builds the additional authenticated data that binds a sealed document to
/// its identity: the 16 raw bytes of the document id UUID followed by the
/// document version as 4 big-endian bytes.
///
/// Sealing a document with this AAD makes decryption fail unless the caller
/// asks for exactly the same document and version. That prevents
/// substituting one document's ciphertext for another's (the id would
/// differ) and replaying an older version of the same document (the version
/// would differ).
pub fn document_aad(id: DocumentId, version: DocumentVersion) -> [u8; DOCUMENT_AAD_LEN] {
    let mut aad = [0u8; DOCUMENT_AAD_LEN];
    aad[..16].copy_from_slice(id.as_uuid().as_bytes());
    aad[16..].copy_from_slice(&version.get().to_be_bytes());
    aad
}

/// Outbound port for authenticated encryption with associated data.
///
/// Key material crosses this interface as plain byte slices so this crate
/// stays free of cryptographic dependencies. Callers keep the owning
/// buffers in zeroizing containers and only lend them here for the duration
/// of a call.
pub trait AuthenticatedCipher {
    /// Encrypts and authenticates `plaintext` under a 32-byte `key`,
    /// binding `aad` into the authentication tag. A fresh random nonce is
    /// drawn for every call.
    fn seal(&self, key: &[u8], aad: &[u8], plaintext: &[u8]) -> Result<SealedPayload, DomainError>;

    /// Decrypts a payload produced by [`AuthenticatedCipher::seal`].
    ///
    /// Fails with [`DomainError::AuthenticationFailed`] when the key, the
    /// payload bytes, or the AAD differ in any way from what was sealed.
    fn open(&self, key: &[u8], aad: &[u8], payload: &SealedPayload)
        -> Result<Vec<u8>, DomainError>;
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn sample_id() -> DocumentId {
        let bytes: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        DocumentId::from_uuid(Uuid::from_bytes(bytes))
    }

    #[test]
    fn sealed_payload_rejects_bytes_shorter_than_nonce_and_tag() {
        let err = SealedPayload::from_bytes(vec![0u8; SEALED_PAYLOAD_MIN_LEN - 1]).unwrap_err();
        assert_eq!(
            err,
            DomainError::MalformedSealedPayload {
                min: SEALED_PAYLOAD_MIN_LEN,
                actual: SEALED_PAYLOAD_MIN_LEN - 1,
            }
        );
    }

    #[test]
    fn sealed_payload_accepts_empty_ciphertext() {
        let payload = SealedPayload::from_bytes(vec![7u8; SEALED_PAYLOAD_MIN_LEN]).unwrap();
        assert_eq!(payload.nonce().len(), AES_GCM_NONCE_LEN);
        assert_eq!(payload.ciphertext_and_tag().len(), AES_GCM_TAG_LEN);
    }

    #[test]
    fn sealed_payload_slices_nonce_and_remainder() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[1u8; AES_GCM_NONCE_LEN]);
        bytes.extend_from_slice(&[2u8; 5]);
        bytes.extend_from_slice(&[3u8; AES_GCM_TAG_LEN]);
        let payload = SealedPayload::from_bytes(bytes.clone()).unwrap();
        assert_eq!(payload.nonce(), &[1u8; AES_GCM_NONCE_LEN]);
        assert_eq!(payload.ciphertext_and_tag(), &bytes[AES_GCM_NONCE_LEN..]);
        assert_eq!(payload.as_bytes(), bytes.as_slice());
        assert_eq!(payload.into_bytes(), bytes);
    }

    #[test]
    fn document_aad_is_id_bytes_then_big_endian_version() {
        let id = sample_id();
        let version = DocumentVersion::new(258).unwrap();
        let aad = document_aad(id, version);
        assert_eq!(&aad[..16], id.as_uuid().as_bytes());
        assert_eq!(&aad[16..], &[0x00, 0x00, 0x01, 0x02]);
    }

    #[test]
    fn document_aad_changes_with_the_version() {
        let id = sample_id();
        let v1 = document_aad(id, DocumentVersion::new(1).unwrap());
        let v2 = document_aad(id, DocumentVersion::new(2).unwrap());
        assert_ne!(v1, v2);
    }

    #[test]
    fn document_aad_changes_with_the_id() {
        let version = DocumentVersion::initial();
        let a = document_aad(DocumentId::new(), version);
        let b = document_aad(DocumentId::new(), version);
        assert_ne!(a, b);
    }
}
