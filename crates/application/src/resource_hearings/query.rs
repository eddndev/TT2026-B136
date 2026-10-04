use super::ResourceHearingCreation;
use crate::ApplicationError;
use domain::{
    cases::CaseId, procedural_resources::ResourceId, resource_hearings::ResourceHearingId,
};

/// Bounded UUID order within one resource, independent of association status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceHearingReadQuery {
    limit: u16,
    after_id: Option<ResourceHearingId>,
}

impl ResourceHearingReadQuery {
    pub fn new(limit: u16, after_id: Option<ResourceHearingId>) -> Result<Self, ApplicationError> {
        if !(1..=20).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "resource hearing list limit must be 1..20".into(),
            ));
        }
        Ok(Self { limit, after_id })
    }

    pub const fn limit(self) -> u16 {
        self.limit
    }

    pub const fn after_id(self) -> Option<ResourceHearingId> {
        self.after_id
    }
}

impl Default for ResourceHearingReadQuery {
    fn default() -> Self {
        Self {
            limit: 10,
            after_id: None,
        }
    }
}

/// Complete historical creations; each association is its initial capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingPage {
    pub case_id: CaseId,
    pub resource_id: ResourceId,
    pub items: Vec<ResourceHearingCreation>,
    pub has_more: bool,
    pub next_after_id: Option<ResourceHearingId>,
}
