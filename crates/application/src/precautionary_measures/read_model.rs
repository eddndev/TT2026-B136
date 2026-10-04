use super::*;
use crate::ApplicationError;
use domain::cases::CaseId;
use domain::precautionary_measures::MeasureDecisionId;

/// Ascending immutable decision UUIDs within one case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureDecisionReadQuery {
    limit: u16,
    after_id: Option<MeasureDecisionId>,
}

impl MeasureDecisionReadQuery {
    pub fn new(limit: u16, after_id: Option<MeasureDecisionId>) -> Result<Self, ApplicationError> {
        if !(1..=20).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "measure decision list limit must be 1..20".into(),
            ));
        }
        Ok(Self { limit, after_id })
    }
    pub const fn limit(self) -> u16 {
        self.limit
    }
    pub const fn after_id(self) -> Option<MeasureDecisionId> {
        self.after_id
    }
}
impl Default for MeasureDecisionReadQuery {
    fn default() -> Self {
        Self {
            limit: 10,
            after_id: None,
        }
    }
}

/// Each item retains its original group and complete dependency closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionPage {
    pub case_id: CaseId,
    pub items: Vec<MeasureDecisionStoredOperation>,
    pub has_more: bool,
    pub next_after_id: Option<MeasureDecisionId>,
}
