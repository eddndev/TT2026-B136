use super::*;
use crate::ApplicationError;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineProfileStatusFilter {
    All,
    #[default]
    Published,
    Retired,
}
impl DeadlineProfileStatusFilter {
    pub const fn status(self) -> Option<DeadlineProfileStatus> {
        match self {
            Self::All => None,
            Self::Published => Some(DeadlineProfileStatus::Published),
            Self::Retired => Some(DeadlineProfileStatus::Retired),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileQuery {
    limit: u32,
    after_id: Option<DeadlineProfileId>,
    status: DeadlineProfileStatusFilter,
}
impl DeadlineProfileQuery {
    pub fn new(
        limit: u32,
        after_id: Option<DeadlineProfileId>,
        status: DeadlineProfileStatusFilter,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit, 100)?;
        Ok(Self {
            limit,
            after_id,
            status,
        })
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn after_id(&self) -> Option<DeadlineProfileId> {
        self.after_id
    }
    pub const fn status(&self) -> DeadlineProfileStatusFilter {
        self.status
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineProfileHistoryQuery {
    limit: u32,
    before_revision: Option<DeadlineProfileRevision>,
}
impl DeadlineProfileHistoryQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        validate_limit(limit, 20)?;
        Ok(Self {
            limit,
            before_revision: before_revision
                .map(DeadlineProfileRevision::new)
                .transpose()?,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn before_revision(self) -> Option<DeadlineProfileRevision> {
        self.before_revision
    }
}
fn validate_limit(limit: u32, max: u32) -> Result<(), ApplicationError> {
    if !(1..=max).contains(&limit) {
        return Err(ApplicationError::InvalidInput(
            "invalid profile page limit".into(),
        ));
    }
    Ok(())
}
