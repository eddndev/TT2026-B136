use super::OwnedMeasureRecord;
use crate::{precautionary_measures::MeasureDecisionRecordHistoryEvidence, ApplicationError};
use domain::{
    cases::CaseId,
    precautionary_hearings::{MeasureId, PrecautionaryMeasureRef},
};

/// Ascending stable measure identities, including heads marked entered in error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureRecordReadQuery {
    limit: u16,
    after_id: Option<MeasureId>,
}

impl MeasureRecordReadQuery {
    pub fn new(limit: u16, after_id: Option<MeasureId>) -> Result<Self, ApplicationError> {
        if !(1..=20).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "measure record list limit must be 1..20".into(),
            ));
        }
        Ok(Self { limit, after_id })
    }

    pub const fn limit(self) -> u16 {
        self.limit
    }

    pub const fn after_id(self) -> Option<MeasureId> {
        self.after_id
    }
}

impl Default for MeasureRecordReadQuery {
    fn default() -> Self {
        Self {
            limit: 10,
            after_id: None,
        }
    }
}

/// Exact captured record and its complete owning group and ancestor closure.
/// Supplied public material requires reconstruction before disclosure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureRecordDetail {
    pub case_id: CaseId,
    pub reference: PrecautionaryMeasureRef,
    pub record: OwnedMeasureRecord,
    pub record_history: MeasureDecisionRecordHistoryEvidence,
}

/// Latest recorded revisions ordered by stable measure identity, not legal status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureRecordPage {
    pub case_id: CaseId,
    pub items: Vec<MeasureRecordDetail>,
    pub has_more: bool,
    pub next_after_id: Option<MeasureId>,
}
