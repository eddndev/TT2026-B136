//! Envelope key management backed by the AES-256-GCM cipher adapter.

use domain::crypto::cipher::AuthenticatedCipher;
use domain::crypto::keys::{KeyManager, WrappedDek, DATA_KEY_LEN};
use domain::DomainError;
use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroizing;

use crate::encryption::RingAesGcmCipher;
use crate::error::CryptoError;

/// Generates data encryption keys and seals them under the key encryption
/// key with AES-256-GCM.
///
/// The wrap uses empty additional authenticated data: a wrapped key is
/// bound to its document by the vault file that carries it, and keeping the
/// wrap independent of document identity lets a key rotation rewrap every
/// stored key without knowing which document each one belongs to.
pub struct EnvelopeKeyManager {
    cipher: RingAesGcmCipher,
    rng: SystemRandom,
}

impl EnvelopeKeyManager {
    /// Creates a manager using the operating system's random source.
    pub fn new() -> Self {
        Self {
            cipher: RingAesGcmCipher::new(),
            rng: SystemRandom::new(),
        }
    }
}

impl Default for EnvelopeKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyManager for EnvelopeKeyManager {
    fn generate_dek(&self) -> Result<Vec<u8>, DomainError> {
        // The caller moves this buffer into a zeroizing container, as the
        // port contract requires; a moved Vec keeps its allocation, so the
        // eventual wipe covers the bytes filled in here.
        let mut dek = vec![0u8; DATA_KEY_LEN];
        self.rng
            .fill(&mut dek)
            .map_err(|_| CryptoError::Backend("csprng failed to produce a data key".to_string()))?;
        Ok(dek)
    }

    fn wrap_dek(&self, kek: &[u8], dek: &[u8]) -> Result<WrappedDek, DomainError> {
        if dek.len() != DATA_KEY_LEN {
            return Err(CryptoError::InvalidKeyMaterial(format!(
                "data key must be {DATA_KEY_LEN} bytes, got {}",
                dek.len()
            ))
            .into());
        }
        let sealed = self.cipher.seal(kek, &[], dek)?;
        Ok(WrappedDek::from_payload(sealed))
    }

    fn unwrap_dek(&self, kek: &[u8], wrapped: &WrappedDek) -> Result<Vec<u8>, DomainError> {
        let mut dek = Zeroizing::new(self.cipher.open(kek, &[], wrapped.payload())?);
        if dek.len() != DATA_KEY_LEN {
            return Err(CryptoError::InvalidKeyMaterial(format!(
                "wrapped payload held {} bytes instead of a {DATA_KEY_LEN}-byte data key",
                dek.len()
            ))
            .into());
        }
        // Hand the buffer back without copying: the caller immediately puts
        // it into its own zeroizing container, and the zeroizing guard here
        // is left holding an empty vector.
        Ok(std::mem::take(&mut *dek))
    }

    fn rewrap_dek(
        &self,
        old_kek: &[u8],
        new_kek: &[u8],
        wrapped: &WrappedDek,
    ) -> Result<WrappedDek, DomainError> {
        let dek = Zeroizing::new(self.unwrap_dek(old_kek, wrapped)?);
        self.wrap_dek(new_kek, &dek)
    }
}

#[cfg(test)]
mod tests {
    use domain::crypto::keys::KEY_ENCRYPTION_KEY_LEN;

    use super::*;

    const KEK: [u8; KEY_ENCRYPTION_KEY_LEN] = [0x41; KEY_ENCRYPTION_KEY_LEN];
    const NEW_KEK: [u8; KEY_ENCRYPTION_KEY_LEN] = [0x42; KEY_ENCRYPTION_KEY_LEN];

    #[test]
    fn generated_dek_has_the_expected_length_and_varies() {
        let manager = EnvelopeKeyManager::new();
        let first = Zeroizing::new(manager.generate_dek().unwrap());
        let second = Zeroizing::new(manager.generate_dek().unwrap());
        assert_eq!(first.len(), DATA_KEY_LEN);
        assert_eq!(second.len(), DATA_KEY_LEN);
        assert_ne!(*first, *second);
    }

    #[test]
    fn wrap_then_unwrap_round_trips_the_dek() {
        let manager = EnvelopeKeyManager::new();
        let dek = Zeroizing::new(manager.generate_dek().unwrap());
        let wrapped = manager.wrap_dek(&KEK, &dek).unwrap();
        let unwrapped = Zeroizing::new(manager.unwrap_dek(&KEK, &wrapped).unwrap());
        assert_eq!(*unwrapped, *dek);
    }

    #[test]
    fn unwrap_with_the_wrong_kek_fails_opaquely() {
        let manager = EnvelopeKeyManager::new();
        let dek = Zeroizing::new(manager.generate_dek().unwrap());
        let wrapped = manager.wrap_dek(&KEK, &dek).unwrap();
        let err = manager.unwrap_dek(&NEW_KEK, &wrapped).unwrap_err();
        assert_eq!(err, DomainError::AuthenticationFailed);
    }

    #[test]
    fn wrap_rejects_a_dek_of_the_wrong_length() {
        let manager = EnvelopeKeyManager::new();
        let err = manager.wrap_dek(&KEK, &[0u8; 16]).unwrap_err();
        assert!(matches!(err, DomainError::InvalidKeyMaterial(_)));
    }

    #[test]
    fn rewrap_moves_the_dek_to_the_new_kek() {
        let manager = EnvelopeKeyManager::new();
        let dek = Zeroizing::new(manager.generate_dek().unwrap());
        let wrapped = manager.wrap_dek(&KEK, &dek).unwrap();

        let rewrapped = manager.rewrap_dek(&KEK, &NEW_KEK, &wrapped).unwrap();

        let recovered = Zeroizing::new(manager.unwrap_dek(&NEW_KEK, &rewrapped).unwrap());
        assert_eq!(*recovered, *dek);
        assert_eq!(
            manager.unwrap_dek(&KEK, &rewrapped).unwrap_err(),
            DomainError::AuthenticationFailed
        );
    }

    #[test]
    fn rotation_keeps_a_document_sealed_under_the_dek_decryptable() {
        let manager = EnvelopeKeyManager::new();
        let cipher = RingAesGcmCipher::new();
        let dek = Zeroizing::new(manager.generate_dek().unwrap());
        let sealed_document = cipher.seal(&dek, b"aad", b"the document").unwrap();
        let wrapped = manager.wrap_dek(&KEK, &dek).unwrap();

        let rewrapped = manager.rewrap_dek(&KEK, &NEW_KEK, &wrapped).unwrap();

        let recovered = Zeroizing::new(manager.unwrap_dek(&NEW_KEK, &rewrapped).unwrap());
        let opened = cipher.open(&recovered, b"aad", &sealed_document).unwrap();
        assert_eq!(opened, b"the document");
    }
}
