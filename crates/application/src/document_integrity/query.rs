use super::{DocumentIntegrityIncident, DocumentIntegrityIncidentId};
use crate::ApplicationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentIntegrityQuery {
    limit: u32,
    after_id: Option<DocumentIntegrityIncidentId>,
}

impl DocumentIntegrityQuery {
    pub fn new(
        limit: u32,
        after_id: Option<DocumentIntegrityIncidentId>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "document integrity page limit must be between 1 and 100".into(),
            ));
        }
        Ok(Self { limit, after_id })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn after_id(self) -> Option<DocumentIntegrityIncidentId> {
        self.after_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentIntegrityPage {
    pub incidents: Vec<DocumentIntegrityIncident>,
    pub has_more: bool,
    pub next_after_id: Option<DocumentIntegrityIncidentId>,
}
