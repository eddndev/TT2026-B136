use super::{HearingResultId, HearingResultRevision, HearingResultStatus};
use crate::ApplicationError;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum HearingResultStatusFilter {
    #[default]
    All,
    Recorded,
    Withdrawn,
}
impl HearingResultStatusFilter {
    pub const fn status(self) -> Option<HearingResultStatus> {
        match self {
            Self::All => None,
            Self::Recorded => Some(HearingResultStatus::Recorded),
            Self::Withdrawn => Some(HearingResultStatus::Withdrawn),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingResultQuery {
    limit: u32,
    after_id: Option<HearingResultId>,
    status: HearingResultStatusFilter,
}
impl HearingResultQuery {
    pub fn new(
        limit: u32,
        after_id: Option<HearingResultId>,
        status: HearingResultStatusFilter,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit, 100)?;
        Ok(Self {
            limit,
            after_id,
            status,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn after_id(self) -> Option<HearingResultId> {
        self.after_id
    }
    pub const fn status(self) -> HearingResultStatusFilter {
        self.status
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingResultHistoryQuery {
    limit: u32,
    before_revision: Option<HearingResultRevision>,
}
impl HearingResultHistoryQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        validate_limit(limit, 20)?;
        Ok(Self {
            limit,
            before_revision: before_revision
                .map(HearingResultRevision::new)
                .transpose()?,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn before_revision(self) -> Option<HearingResultRevision> {
        self.before_revision
    }
}
fn validate_limit(limit: u32, max: u32) -> Result<(), ApplicationError> {
    if !(1..=max).contains(&limit) {
        return Err(ApplicationError::InvalidInput(
            "invalid hearing result page limit".into(),
        ));
    }
    Ok(())
}
