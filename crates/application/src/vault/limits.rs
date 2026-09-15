//! Allocation-free admission checks for the AES-256-GCM vault profile.

use domain::crypto::{cipher::SEALED_PAYLOAD_MIN_LEN, keys::DATA_KEY_LEN};

use super::{VAULT_HEADER_LEN, VAULT_MAGIC};
use crate::ApplicationError;

const WRAPPED_AES_KEY_LEN: usize = DATA_KEY_LEN + SEALED_PAYLOAD_MIN_LEN;
const AES_VAULT_OVERHEAD: usize = VAULT_HEADER_LEN + WRAPPED_AES_KEY_LEN + SEALED_PAYLOAD_MIN_LEN;

/// A plaintext bound together with the exact AES envelope overhead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultReadLimits {
    max_plaintext_bytes: usize,
    max_vault_bytes: usize,
}

impl VaultReadLimits {
    pub fn aes256_gcm(max_plaintext_bytes: usize) -> Result<Self, ApplicationError> {
        let max_vault_bytes = max_plaintext_bytes
            .checked_add(AES_VAULT_OVERHEAD)
            .ok_or_else(|| {
                ApplicationError::InvalidConfiguration("vault read limit overflows usize".into())
            })?;
        Ok(Self {
            max_plaintext_bytes,
            max_vault_bytes,
        })
    }

    pub const fn max_plaintext_bytes(&self) -> usize {
        self.max_plaintext_bytes
    }

    pub const fn max_vault_bytes(&self) -> usize {
        self.max_vault_bytes
    }

    /// Checks stored bytes before the generic parser copies encrypted fields.
    pub fn inspect(&self, vault: &[u8]) -> Result<usize, ApplicationError> {
        self.inspect_header(vault, vault.len())
    }

    /// Checks a header probe and a scalar length before storage transfers a blob.
    /// Storage must gate the final read again if bytes can change after its probe.
    pub fn inspect_header(
        &self,
        header: &[u8],
        total_bytes: usize,
    ) -> Result<usize, ApplicationError> {
        if total_bytes > self.max_vault_bytes {
            return Err(ApplicationError::StageSupportTooLarge);
        }
        if total_bytes < AES_VAULT_OVERHEAD || header.len() < VAULT_HEADER_LEN {
            return Err(ApplicationError::MalformedVaultFile(
                "truncated AES vault".into(),
            ));
        }
        if &header[..VAULT_MAGIC.len()] != VAULT_MAGIC {
            return Err(ApplicationError::MalformedVaultFile(
                "invalid vault magic".into(),
            ));
        }
        let mut encoded_length = [0u8; 4];
        encoded_length.copy_from_slice(&header[VAULT_MAGIC.len()..VAULT_HEADER_LEN]);
        if u32::from_be_bytes(encoded_length) != WRAPPED_AES_KEY_LEN as u32 {
            return Err(ApplicationError::MalformedVaultFile(
                "wrapped AES data key must contain 60 bytes".into(),
            ));
        }
        Ok(total_bytes - AES_VAULT_OVERHEAD)
    }
}
