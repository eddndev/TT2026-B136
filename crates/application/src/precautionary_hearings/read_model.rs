use super::*;
use crate::ApplicationError;
use domain::cases::CaseId;
use domain::precautionary_hearings::PrecautionaryHearingId;

/// Ascending appointment UUIDs within one case, including cancelled records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrecautionaryHearingReadQuery {
    limit: u16,
    after_id: Option<PrecautionaryHearingId>,
}

impl PrecautionaryHearingReadQuery {
    pub fn new(
        limit: u16,
        after_id: Option<PrecautionaryHearingId>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=20).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "precautionary hearing list limit must be 1..20".into(),
            ));
        }
        Ok(Self { limit, after_id })
    }
    pub const fn limit(self) -> u16 {
        self.limit
    }
    pub const fn after_id(self) -> Option<PrecautionaryHearingId> {
        self.after_id
    }
}
impl Default for PrecautionaryHearingReadQuery {
    fn default() -> Self {
        Self {
            limit: 10,
            after_id: None,
        }
    }
}

/// Each item retains its exact head and the complete original prefix to that head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingPage {
    pub case_id: CaseId,
    pub items: Vec<PrecautionaryHearingStoredOperation>,
    pub has_more: bool,
    pub next_after_id: Option<PrecautionaryHearingId>,
}
