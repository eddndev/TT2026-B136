use super::{FactRevision, FactStatus, NotificationId, ResolutionId};
use crate::ApplicationError;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FactStatusFilter {
    #[default]
    All,
    Recorded,
    Withdrawn,
}
impl FactStatusFilter {
    pub const fn status(self) -> Option<FactStatus> {
        match self {
            Self::All => None,
            Self::Recorded => Some(FactStatus::Recorded),
            Self::Withdrawn => Some(FactStatus::Withdrawn),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactListQuery<I> {
    limit: u32,
    after_id: Option<I>,
    status: FactStatusFilter,
}
pub type ResolutionQuery = FactListQuery<ResolutionId>;
pub type NotificationQuery = FactListQuery<NotificationId>;
impl<I: Copy> FactListQuery<I> {
    pub fn new(
        limit: u32,
        after_id: Option<I>,
        status: FactStatusFilter,
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
    pub const fn after_id(self) -> Option<I> {
        self.after_id
    }
    pub const fn status(self) -> FactStatusFilter {
        self.status
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactHistoryQuery {
    limit: u32,
    before_revision: Option<FactRevision>,
}
impl FactHistoryQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        validate_limit(limit, 20)?;
        Ok(Self {
            limit,
            before_revision: before_revision.map(FactRevision::new).transpose()?,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn before_revision(self) -> Option<FactRevision> {
        self.before_revision
    }
}
fn validate_limit(limit: u32, max: u32) -> Result<(), ApplicationError> {
    if !(1..=max).contains(&limit) {
        return Err(ApplicationError::InvalidInput(
            "invalid procedural fact page limit".into(),
        ));
    }
    Ok(())
}
