use super::{
    ResourceActivityId, ResourceActivityQuery, ResourceActivityStatus, ResourceActivityView,
};
use crate::ApplicationError;
use domain::{clock::OffsetDateTime, deadlines::DeadlineId, hearings::HearingId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceActivityTargetId {
    Hearing(HearingId),
    Deadline(DeadlineId),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceActivityTargetQuery(ResourceActivityQuery);
impl ResourceActivityTargetQuery {
    pub fn new(
        limit: u32,
        after_id: Option<ResourceActivityId>,
        status: Option<ResourceActivityStatus>,
    ) -> Result<Self, ApplicationError> {
        ResourceActivityQuery::new(limit, after_id, None, status).map(Self)
    }
    pub const fn limit(self) -> u32 {
        self.0.limit()
    }
    pub const fn after_id(self) -> Option<ResourceActivityId> {
        self.0.after_id()
    }
    pub const fn status(self) -> Option<ResourceActivityStatus> {
        self.0.status()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivityTargetPage {
    pub checked_at: OffsetDateTime,
    pub associations: Vec<ResourceActivityView>,
    pub has_more: bool,
    pub next_after_id: Option<ResourceActivityId>,
}
