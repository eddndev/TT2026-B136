//! Organizational values belonging to a document series, outside its evidence.

use serde::Serialize;

use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocumentMetadata {
    document_type: Option<String>,
    classification: Option<String>,
    tags: Vec<String>,
}

impl DocumentMetadata {
    pub fn new(
        document_type: Option<&str>,
        classification: Option<&str>,
        tags: &[String],
    ) -> Result<Self, DomainError> {
        if tags.len() > 20 {
            return Err(invalid("tags", "at most 20 entries are allowed"));
        }
        let document_type = optional(document_type, "document_type")?;
        let classification = optional(classification, "classification")?;
        let mut tags = tags
            .iter()
            .map(|tag| {
                let value = normalized(tag, "tags", 40)?;
                if value.is_empty() {
                    return Err(invalid("tags", "entries must not be empty"));
                }
                Ok(value)
            })
            .collect::<Result<Vec<_>, _>>()?;
        tags.sort_unstable_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        tags.dedup();
        Ok(Self {
            document_type,
            classification,
            tags,
        })
    }

    pub fn empty() -> Self {
        Self {
            document_type: None,
            classification: None,
            tags: Vec::new(),
        }
    }

    pub fn document_type(&self) -> Option<&str> {
        self.document_type.as_deref()
    }

    pub fn classification(&self) -> Option<&str> {
        self.classification.as_deref()
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Length-prefixed UTF-8, with explicit optional fields and sorted tags.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"DMETA1".to_vec();
        for value in [self.document_type(), self.classification()] {
            match value {
                Some(value) => {
                    bytes.push(1);
                    write_value(&mut bytes, value);
                }
                None => bytes.push(0),
            }
        }
        bytes.extend_from_slice(&(self.tags.len() as u32).to_be_bytes());
        for tag in &self.tags {
            write_value(&mut bytes, tag);
        }
        bytes
    }
}

fn optional(value: Option<&str>, field: &'static str) -> Result<Option<String>, DomainError> {
    value
        .map(|value| normalized(value, field, 80))
        .transpose()
        .map(|value| value.filter(|value| !value.is_empty()))
}

fn normalized(value: &str, field: &'static str, limit: usize) -> Result<String, DomainError> {
    if value.chars().any(char::is_control) {
        return Err(invalid(field, "control characters are not allowed"));
    }
    let value = value.trim();
    if value.chars().count() > limit {
        return Err(invalid(field, "character limit exceeded"));
    }
    Ok(value.to_owned())
}

fn invalid(field: &'static str, reason: &'static str) -> DomainError {
    DomainError::InvalidDocumentMetadata { field, reason }
}

fn write_value(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

/// Zero denotes no recorded classification; stored revisions start at one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct MetadataRevision(u32);

impl MetadataRevision {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn unclassified() -> Self {
        Self(0)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}
