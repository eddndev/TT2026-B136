//! Bounded metadata queries for authorized documents within one case.

use super::CaseDocumentSummary;
use crate::ApplicationError;

/// A validated page and optional literal name and sealed-state filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentQuery {
    limit: u32,
    offset: u32,
    name: Option<String>,
    sealed: Option<bool>,
}

impl DocumentQuery {
    pub fn new(
        limit: u32,
        offset: u32,
        name: Option<&str>,
        sealed: Option<bool>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "document limit must be between 1 and 100".into(),
            ));
        }
        if name.is_some_and(|value| value.chars().any(char::is_control)) {
            return Err(ApplicationError::InvalidInput(
                "document name search must not contain control characters".into(),
            ));
        }
        let name = name.map(str::trim).filter(|value| !value.is_empty());
        if name.is_some_and(|value| value.chars().count() > 200) {
            return Err(ApplicationError::InvalidInput(
                "document name search must contain at most 200 characters".into(),
            ));
        }
        Ok(Self {
            limit,
            offset,
            name: name.map(str::to_owned),
            sealed,
        })
    }

    pub const fn limit(&self) -> u32 {
        self.limit
    }

    pub const fn offset(&self) -> u32 {
        self.offset
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub const fn sealed(&self) -> Option<bool> {
        self.sealed
    }
}

/// One page of metadata with an indicator for the next page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPage {
    pub documents: Vec<CaseDocumentSummary>,
    pub has_more: bool,
}
