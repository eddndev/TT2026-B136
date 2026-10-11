//! AES-256-GCM protection for persisted invitation delivery and MFA material.

use application::identity::invitations::{
    InvitationProtectionContext, InvitationProtector, InvitationPurpose,
    MAX_INVITATION_PAYLOAD_BYTES,
};
use application::ApplicationError;
use domain::crypto::cipher::{AES_256_GCM_KEY_LEN, SEALED_PAYLOAD_MIN_LEN};
use domain::crypto::{AuthenticatedCipher, SealedPayload};
use zeroize::Zeroizing;

use crate::RingAesGcmCipher;

/// Protects invitation material under a runtime key with purpose-bound context.
///
/// The persisted format is the cipher's nonce, ciphertext and authentication tag.
/// AAD is `qadra:invitation-protection:v1\0`, followed by a purpose byte
/// (delivery = 1, pending MFA = 2), UUID bytes, and a big-endian u64 revision.
/// This format is distinct from the persisted user TOTP protection context.
pub struct AesGcmInvitationProtector {
    cipher: RingAesGcmCipher,
    kek: Zeroizing<Vec<u8>>,
}

impl AesGcmInvitationProtector {
    pub fn new(kek: Zeroizing<Vec<u8>>) -> Result<Self, ApplicationError> {
        if kek.len() != AES_256_GCM_KEY_LEN {
            return Err(ApplicationError::InvalidConfiguration(
                "invitation protection requires an AES-256 key".into(),
            ));
        }
        Ok(Self {
            cipher: RingAesGcmCipher::new(),
            kek,
        })
    }
}

impl InvitationProtector for AesGcmInvitationProtector {
    fn protect(
        &self,
        context: InvitationProtectionContext,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, ApplicationError> {
        let aad = invitation_aad(context)?;
        if plaintext.len() > MAX_INVITATION_PAYLOAD_BYTES {
            return Err(payload_limit_error());
        }
        self.cipher
            .seal(&self.kek, &aad, plaintext)
            .map(SealedPayload::into_bytes)
            .map_err(|_| ApplicationError::Port("invitation protection failed".into()))
    }

    fn expose(
        &self,
        context: InvitationProtectionContext,
        protected: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
        let aad = invitation_aad(context)?;
        if protected.len() > MAX_INVITATION_PAYLOAD_BYTES + SEALED_PAYLOAD_MIN_LEN {
            return Err(payload_limit_error());
        }
        let payload =
            SealedPayload::from_bytes(protected.to_vec()).map_err(|_| exposure_error())?;
        self.cipher
            .open(&self.kek, &aad, &payload)
            .map(Zeroizing::new)
            .map_err(|_| exposure_error())
    }
}

fn invitation_aad(context: InvitationProtectionContext) -> Result<Vec<u8>, ApplicationError> {
    if context.id.as_uuid().is_nil() || context.revision == 0 || context.revision > i64::MAX as u64
    {
        return Err(ApplicationError::InvalidInput(
            "invalid invitation protection context".into(),
        ));
    }
    let mut aad = b"qadra:invitation-protection:v1\0".to_vec();
    aad.push(match context.purpose {
        InvitationPurpose::Delivery => 1,
        InvitationPurpose::PendingMfa => 2,
    });
    aad.extend_from_slice(context.id.as_uuid().as_bytes());
    aad.extend_from_slice(&context.revision.to_be_bytes());
    Ok(aad)
}

fn payload_limit_error() -> ApplicationError {
    ApplicationError::InvalidInput("invitation protection payload exceeds its limit".into())
}

fn exposure_error() -> ApplicationError {
    ApplicationError::Port("invitation protected payload could not be opened".into())
}
