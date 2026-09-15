//! Exact, case-sensitive filters on current organizational values.

use super::DocumentMetadata;
use crate::ApplicationError;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DocumentMetadataFilter {
    document_type: Option<String>,
    classification: Option<String>,
    tag: Option<String>,
}

impl DocumentMetadataFilter {
    pub fn new(
        document_type: Option<&str>,
        classification: Option<&str>,
        tag: Option<&str>,
    ) -> Result<Self, ApplicationError> {
        let tags: Vec<String> = tag.into_iter().map(str::to_owned).collect();
        let normalized = DocumentMetadata::new(document_type, classification, &tags)
            .map_err(|error| ApplicationError::InvalidInput(error.to_string()))?;
        Ok(Self {
            document_type: normalized.document_type().map(str::to_owned),
            classification: normalized.classification().map(str::to_owned),
            tag: normalized.tags().first().cloned(),
        })
    }

    pub fn document_type(&self) -> Option<&str> {
        self.document_type.as_deref()
    }

    pub fn classification(&self) -> Option<&str> {
        self.classification.as_deref()
    }

    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }
}
