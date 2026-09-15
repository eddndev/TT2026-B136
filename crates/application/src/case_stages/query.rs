use super::CaseStageRevision;
use crate::ApplicationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseStageExpectation {
    Unregistered,
    Revision(CaseStageRevision),
}

impl CaseStageExpectation {
    pub fn new(value: u32) -> Self {
        match value {
            0 => Self::Unregistered,
            _ => Self::Revision(CaseStageRevision::new(value).expect("positive stage revision")),
        }
    }
    pub const fn get(self) -> u32 {
        match self {
            Self::Unregistered => 0,
            Self::Revision(revision) => revision.get(),
        }
    }
}

/// Descending revision pagination across initial registration and later changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseStageQuery {
    limit: u32,
    before_revision: Option<CaseStageRevision>,
}

impl CaseStageQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "stage history limit must be between 1 and 100".into(),
            ));
        }
        let before_revision = before_revision
            .map(CaseStageRevision::new)
            .transpose()
            .map_err(|error| ApplicationError::InvalidInput(error.to_string()))?;
        Ok(Self {
            limit,
            before_revision,
        })
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn before_revision(&self) -> Option<CaseStageRevision> {
        self.before_revision
    }
}
