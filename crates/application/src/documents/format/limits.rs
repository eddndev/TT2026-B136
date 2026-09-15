//! Size policy for new stage supports, independent of general historical reads.

use domain::crypto::{archive::MAX_ENTRY_NAME_LEN, ArchiveEntry};

use super::super::DocumentRecord;
use crate::{vault::VaultReadLimits, ApplicationError};

const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_EVIDENCE_BYTES: usize = 1024 * 1024;
const MAX_DOCUMENTS: usize = 2;

/// Fixed admission policy supplied by composition, never by an HTTP caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageSupportReadLimits {
    vault: VaultReadLimits,
}

impl StageSupportReadLimits {
    pub fn standard() -> Self {
        Self {
            vault: VaultReadLimits::aes256_gcm(MAX_DOCUMENT_BYTES)
                .expect("fixed vault limit fits usize"),
        }
    }

    pub const fn vault(&self) -> &VaultReadLimits {
        &self.vault
    }
    pub const fn max_documents(&self) -> usize {
        MAX_DOCUMENTS
    }
    pub const fn max_batch_plaintext_bytes(&self) -> usize {
        MAX_DOCUMENT_BYTES * MAX_DOCUMENTS
    }
    pub const fn max_evidence_json_bytes(&self) -> usize {
        MAX_EVIDENCE_BYTES
    }

    /// Checks typed sizes before cryptographic parsing. Storage must bound the JSON
    /// before decoding it: the serialized size cannot be recovered from this record.
    pub fn check_record(&self, record: &DocumentRecord) -> Result<usize, ApplicationError> {
        let plaintext_len = self.vault.inspect(&record.vault)?;
        if record.name.len() > MAX_ENTRY_NAME_LEN {
            return Err(ApplicationError::StoredDocumentInconsistent(
                "stored document name exceeds its bound".into(),
            ));
        }
        ArchiveEntry::new(record.name.clone(), Vec::new())?;
        if let Some(evidence) = &record.evidence {
            let lengths = [
                evidence.signature.len(),
                evidence.timestamp_token.len(),
                evidence.signer_certificate_pem.len(),
                evidence.issuer_certificate_pem.len(),
                evidence.crl_pem.len(),
                evidence.tsa_chain_pem.as_ref().map_or(0, Vec::len),
                evidence.openssl_version.len(),
            ];
            let total = lengths
                .iter()
                .try_fold(0usize, |total, length| total.checked_add(*length))
                .ok_or(ApplicationError::StageSupportTooLarge)?;
            if total > MAX_EVIDENCE_BYTES {
                return Err(ApplicationError::StageSupportTooLarge);
            }
        }
        Ok(plaintext_len)
    }
}

impl Default for StageSupportReadLimits {
    fn default() -> Self {
        Self::standard()
    }
}
