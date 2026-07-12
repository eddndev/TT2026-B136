//! Envelope key management port.
//!
//! Each document is encrypted under its own random data encryption key
//! (DEK). The DEK is itself sealed under a key encryption key (KEK) held by
//! the operator, and only the sealed form is ever stored. Rotating the KEK
//! therefore rewraps stored DEKs without re-encrypting any document.

use crate::crypto::cipher::SealedPayload;
use crate::error::DomainError;

/// Byte length of a data encryption key (a 256-bit AES key).
pub const DATA_KEY_LEN: usize = 32;

/// Byte length of the key encryption key (a 256-bit AES key).
pub const KEY_ENCRYPTION_KEY_LEN: usize = 32;

/// A data encryption key sealed under the key encryption key.
///
/// The bytes follow the `nonce || ciphertext || tag` layout documented on
/// [`SealedPayload`]. Only the sealed form is stored or transported; the
/// clear data key exists solely inside zeroizing buffers while in use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedDek(SealedPayload);

impl WrappedDek {
    /// Validates and wraps stored bytes as a wrapped data key.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, DomainError> {
        Ok(Self(SealedPayload::from_bytes(bytes)?))
    }

    /// Wraps a payload freshly produced by a sealing operation.
    pub fn from_payload(payload: SealedPayload) -> Self {
        Self(payload)
    }

    /// Borrows the sealed payload holding the wrapped key.
    pub fn payload(&self) -> &SealedPayload {
        &self.0
    }

    /// Borrows the full `nonce || ciphertext || tag` byte sequence.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Consumes the wrapped key, returning its byte sequence.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

/// Outbound port for generating and (re)wrapping data encryption keys.
///
/// Key material crosses this interface as plain byte buffers so this crate
/// stays free of cryptographic dependencies. Callers move returned buffers
/// into zeroizing containers immediately; moving a `Vec` keeps its heap
/// allocation, so the eventual wipe covers the bytes written here.
pub trait KeyManager {
    /// Returns a fresh random data encryption key of [`DATA_KEY_LEN`] bytes.
    fn generate_dek(&self) -> Result<Vec<u8>, DomainError>;

    /// Seals `dek` under `kek`.
    fn wrap_dek(&self, kek: &[u8], dek: &[u8]) -> Result<WrappedDek, DomainError>;

    /// Recovers the data key sealed inside `wrapped`.
    ///
    /// Fails with [`DomainError::AuthenticationFailed`] when `kek` is not
    /// the key that wrapped it or the wrapped bytes were altered.
    fn unwrap_dek(&self, kek: &[u8], wrapped: &WrappedDek) -> Result<Vec<u8>, DomainError>;

    /// Rewraps under `new_kek` a data key currently sealed under `old_kek`,
    /// leaving everything encrypted with the data key untouched.
    fn rewrap_dek(
        &self,
        old_kek: &[u8],
        new_kek: &[u8],
        wrapped: &WrappedDek,
    ) -> Result<WrappedDek, DomainError>;
}

#[cfg(test)]
mod tests {
    use crate::crypto::cipher::SEALED_PAYLOAD_MIN_LEN;

    use super::*;

    #[test]
    fn wrapped_dek_rejects_bytes_shorter_than_a_sealed_payload() {
        let err = WrappedDek::from_bytes(vec![0u8; SEALED_PAYLOAD_MIN_LEN - 1]).unwrap_err();
        assert_eq!(
            err,
            DomainError::MalformedSealedPayload {
                min: SEALED_PAYLOAD_MIN_LEN,
                actual: SEALED_PAYLOAD_MIN_LEN - 1,
            }
        );
    }

    #[test]
    fn wrapped_dek_round_trips_its_bytes() {
        let bytes = vec![5u8; SEALED_PAYLOAD_MIN_LEN + 32];
        let wrapped = WrappedDek::from_bytes(bytes.clone()).unwrap();
        assert_eq!(wrapped.as_bytes(), bytes.as_slice());
        assert_eq!(wrapped.into_bytes(), bytes);
    }

    #[test]
    fn wrapped_dek_from_payload_preserves_the_payload() {
        let payload = SealedPayload::from_bytes(vec![9u8; SEALED_PAYLOAD_MIN_LEN]).unwrap();
        let wrapped = WrappedDek::from_payload(payload.clone());
        assert_eq!(wrapped.payload(), &payload);
    }
}
