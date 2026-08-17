//! Application records persisted for the local document workflow.

use domain::crypto::{ArchiveEntry, DocumentId, DocumentVersion, Sha256Digest};

use crate::ApplicationError;

/// Cryptographic material captured when a document is sealed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedEvidence {
    pub signature: Vec<u8>,
    pub timestamp_token: Vec<u8>,
    pub signer_certificate_pem: Vec<u8>,
    pub issuer_certificate_pem: Vec<u8>,
    pub crl_pem: Vec<u8>,
    pub tsa_chain_pem: Option<Vec<u8>>,
    pub openssl_version: String,
}

/// One encrypted document version and its optional sealed evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentRecord {
    pub id: DocumentId,
    pub version: DocumentVersion,
    pub name: String,
    pub digest: Sha256Digest,
    pub vault: Vec<u8>,
    pub evidence: Option<SealedEvidence>,
}

/// Public metadata returned by document workflow operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentSummary {
    pub id: DocumentId,
    pub version: DocumentVersion,
    pub name: String,
    pub digest_hex: String,
    pub sealed: bool,
}

impl From<&DocumentRecord> for DocumentSummary {
    fn from(record: &DocumentRecord) -> Self {
        Self {
            id: record.id,
            version: record.version,
            name: record.name.clone(),
            digest_hex: record.digest.to_hex(),
            sealed: record.is_sealed(),
        }
    }
}

/// Downloadable evidence archive and its safe file name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceExport {
    pub archive: Vec<u8>,
    pub file_name: String,
    pub document_digest_hex: String,
}

impl DocumentRecord {
    /// Builds an unsealed record after validating its evidence-archive name.
    pub fn pending(
        id: DocumentId,
        version: DocumentVersion,
        name: String,
        digest: Sha256Digest,
        vault: Vec<u8>,
    ) -> Result<Self, ApplicationError> {
        ArchiveEntry::new(name.clone(), Vec::new())?;
        Ok(Self {
            id,
            version,
            name,
            digest,
            vault,
            evidence: None,
        })
    }

    /// True after signature and timestamp evidence has been attached.
    pub fn is_sealed(&self) -> bool {
        self.evidence.is_some()
    }

    /// Attaches evidence exactly once.
    pub fn seal(&mut self, evidence: SealedEvidence) -> Result<(), ApplicationError> {
        if self.is_sealed() {
            return Err(ApplicationError::DocumentAlreadySealed(self.id.to_string()));
        }
        self.evidence = Some(evidence);
        Ok(())
    }
}
