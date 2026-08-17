//! AES-256-GCM protection for persisted TOTP secrets.

use application::identity::SecretProtector;
use application::ApplicationError;
use domain::crypto::cipher::AES_256_GCM_KEY_LEN;
use domain::crypto::{AuthenticatedCipher, SealedPayload};
use domain::identity::UserId;
use zeroize::Zeroizing;

use crate::RingAesGcmCipher;

/// Encrypts TOTP secrets under a runtime key and binds them to one user ID.
pub struct AesGcmSecretProtector {
    cipher: RingAesGcmCipher,
    kek: Zeroizing<Vec<u8>>,
}

impl AesGcmSecretProtector {
    pub fn new(kek: Zeroizing<Vec<u8>>) -> Result<Self, ApplicationError> {
        if kek.len() != AES_256_GCM_KEY_LEN {
            return Err(ApplicationError::InvalidConfiguration(format!(
                "identity kek must be {AES_256_GCM_KEY_LEN} bytes, got {}",
                kek.len()
            )));
        }
        Ok(Self {
            cipher: RingAesGcmCipher::new(),
            kek,
        })
    }
}

impl SecretProtector for AesGcmSecretProtector {
    fn protect(&self, id: UserId, secret: &[u8]) -> Result<Vec<u8>, ApplicationError> {
        Ok(self
            .cipher
            .seal(&self.kek, &totp_aad(id), secret)?
            .into_bytes())
    }

    fn expose(&self, id: UserId, protected: &[u8]) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
        let payload = SealedPayload::from_bytes(protected.to_vec())?;
        Ok(Zeroizing::new(self.cipher.open(
            &self.kek,
            &totp_aad(id),
            &payload,
        )?))
    }
}

fn totp_aad(id: UserId) -> Vec<u8> {
    let mut aad = b"identity-totp-v1".to_vec();
    aad.extend_from_slice(id.as_uuid().as_bytes());
    aad
}
