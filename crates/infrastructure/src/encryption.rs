//! AES-256-GCM cipher adapter backed by the `ring` crate.

use domain::crypto::cipher::{
    AuthenticatedCipher, SealedPayload, AES_256_GCM_KEY_LEN, AES_GCM_NONCE_LEN, AES_GCM_TAG_LEN,
};
use domain::DomainError;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};

use crate::error::CryptoError;

/// AES-256-GCM cipher that draws a fresh 96-bit nonce from the system
/// CSPRNG for every seal operation and emits the
/// `nonce || ciphertext || tag` layout documented on [`SealedPayload`].
///
/// `ring` names its caller-managed-nonce AEAD type `LessSafeKey` because
/// nonce discipline is left to the caller; here every nonce is freshly
/// random per seal, which is the discipline that type expects.
pub struct RingAesGcmCipher {
    rng: SystemRandom,
}

impl RingAesGcmCipher {
    /// Creates a cipher using the operating system's random source.
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }

    /// Seals with a caller-chosen nonce.
    ///
    /// Only known-answer tests need a fixed nonce; production code uses
    /// [`AuthenticatedCipher::seal`], which draws a fresh random nonce for
    /// every call.
    pub fn seal_with_nonce(
        &self,
        key: &[u8],
        nonce: &[u8; AES_GCM_NONCE_LEN],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<SealedPayload, CryptoError> {
        let key = build_key(key)?;
        let mut in_out = Vec::with_capacity(plaintext.len() + AES_GCM_TAG_LEN);
        in_out.extend_from_slice(plaintext);
        key.seal_in_place_append_tag(
            Nonce::assume_unique_for_key(*nonce),
            Aad::from(aad),
            &mut in_out,
        )
        .map_err(|_| CryptoError::Backend("aead seal rejected the input".to_string()))?;

        let mut bytes = Vec::with_capacity(AES_GCM_NONCE_LEN + in_out.len());
        bytes.extend_from_slice(nonce);
        bytes.extend_from_slice(&in_out);
        SealedPayload::from_bytes(bytes).map_err(|err| CryptoError::Backend(err.to_string()))
    }
}

impl Default for RingAesGcmCipher {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthenticatedCipher for RingAesGcmCipher {
    fn seal(&self, key: &[u8], aad: &[u8], plaintext: &[u8]) -> Result<SealedPayload, DomainError> {
        let mut nonce = [0u8; AES_GCM_NONCE_LEN];
        self.rng
            .fill(&mut nonce)
            .map_err(|_| CryptoError::Backend("csprng failed to produce a nonce".to_string()))?;
        Ok(self.seal_with_nonce(key, &nonce, aad, plaintext)?)
    }

    fn open(
        &self,
        key: &[u8],
        aad: &[u8],
        payload: &SealedPayload,
    ) -> Result<Vec<u8>, DomainError> {
        let key = build_key(key)?;
        let nonce = Nonce::try_assume_unique_for_key(payload.nonce())
            .map_err(|_| CryptoError::DecryptionFailed)?;
        let mut in_out = payload.ciphertext_and_tag().to_vec();
        let plaintext_len = key
            .open_in_place(nonce, Aad::from(aad), &mut in_out)
            .map_err(|_| CryptoError::DecryptionFailed)?
            .len();
        in_out.truncate(plaintext_len);
        Ok(in_out)
    }
}

/// Builds a single-use AES-256-GCM key, rejecting wrong key lengths.
fn build_key(key: &[u8]) -> Result<LessSafeKey, CryptoError> {
    if key.len() != AES_256_GCM_KEY_LEN {
        return Err(CryptoError::InvalidKeyMaterial(format!(
            "aes-256-gcm key must be {AES_256_GCM_KEY_LEN} bytes, got {}",
            key.len()
        )));
    }
    UnboundKey::new(&AES_256_GCM, key)
        .map(LessSafeKey::new)
        .map_err(|_| CryptoError::InvalidKeyMaterial("aead backend rejected the key".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_rejects_a_short_key() {
        let cipher = RingAesGcmCipher::new();
        let err = cipher.seal(&[0u8; 16], b"", b"data").unwrap_err();
        assert!(matches!(err, DomainError::InvalidKeyMaterial(_)));
    }

    #[test]
    fn open_rejects_a_short_key() {
        let cipher = RingAesGcmCipher::new();
        let sealed = cipher.seal(&[0u8; 32], b"", b"data").unwrap();
        let err = cipher.open(&[0u8; 16], b"", &sealed).unwrap_err();
        assert!(matches!(err, DomainError::InvalidKeyMaterial(_)));
    }

    #[test]
    fn seal_then_open_round_trips() {
        let cipher = RingAesGcmCipher::new();
        let key = [0x42u8; 32];
        let sealed = cipher.seal(&key, b"aad", b"confidential").unwrap();
        let opened = cipher.open(&key, b"aad", &sealed).unwrap();
        assert_eq!(opened, b"confidential");
    }
}
