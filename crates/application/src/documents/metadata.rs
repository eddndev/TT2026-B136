//! Current organizational values and their independent revision history.

use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::UserId;

use super::CaseDocumentSummary;
use crate::ApplicationError;
pub use domain::document_metadata::{DocumentMetadata, MetadataRevision};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentDocumentMetadata {
    pub metadata_revision: MetadataRevision,
    pub values: DocumentMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataActorSnapshot {
    pub id: UserId,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMetadataRevision {
    pub metadata_revision: MetadataRevision,
    pub values: DocumentMetadata,
    pub metadata_digest: Sha256Digest,
    pub changed_at: OffsetDateTime,
    pub changed_by: MetadataActorSnapshot,
}

/// Latest content and current classification from the same transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentOverview {
    pub content: CaseDocumentSummary,
    pub current_metadata: CurrentDocumentMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataPage {
    pub revisions: Vec<DocumentMetadataRevision>,
    pub has_more: bool,
    pub next_before_revision: Option<MetadataRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataQuery {
    limit: u32,
    before_revision: Option<MetadataRevision>,
}

impl MetadataQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&limit) || before_revision == Some(0) {
            return Err(ApplicationError::InvalidInput(
                "metadata limit must be between 1 and 100 and before_revision must be positive"
                    .into(),
            ));
        }
        Ok(Self {
            limit,
            before_revision: before_revision.map(MetadataRevision::new),
        })
    }

    pub const fn limit(&self) -> u32 {
        self.limit
    }

    pub const fn before_revision(&self) -> Option<MetadataRevision> {
        self.before_revision
    }
}

/// Hashes the public bounded encoding through the configured SHA-256 port.
pub fn metadata_digest(hasher: &dyn DocumentHasher, metadata: &DocumentMetadata) -> Sha256Digest {
    hasher.hash_bytes(&metadata.canonical_bytes())
}
