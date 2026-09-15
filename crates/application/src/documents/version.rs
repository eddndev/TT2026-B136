//! Version selection and bounded, descending document history queries.

use domain::crypto::DocumentVersion;

use super::CaseDocumentSummary;
use crate::ApplicationError;

/// Resolution policy used once before operating on an exact snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionSelection {
    Current,
    Only,
    Exact(DocumentVersion),
}

/// Descending history page with an exclusive version cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionQuery {
    limit: u32,
    before_version: Option<DocumentVersion>,
}

impl VersionQuery {
    pub fn new(limit: u32, before_version: Option<u32>) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&limit) || before_version == Some(0) {
            return Err(ApplicationError::InvalidInput(
                "history limit must be between 1 and 100 and before_version must be positive"
                    .into(),
            ));
        }
        Ok(Self {
            limit,
            before_version: before_version.map(DocumentVersion::new).transpose()?,
        })
    }

    pub const fn limit(&self) -> u32 {
        self.limit
    }

    pub const fn before_version(&self) -> Option<DocumentVersion> {
        self.before_version
    }
}

/// Historical snapshots and the earliest version actually available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionPage {
    pub versions: Vec<CaseDocumentSummary>,
    pub has_more: bool,
    pub next_before_version: Option<DocumentVersion>,
    pub first_available_version: DocumentVersion,
}
