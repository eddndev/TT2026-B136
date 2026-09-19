use super::*;
use crate::ApplicationError;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceActivityQuery {
    limit: u32,
    after_id: Option<ResourceActivityId>,
    kind: Option<ResourceActivityKind>,
    status: Option<ResourceActivityStatus>,
}
impl ResourceActivityQuery {
    pub fn new(
        limit: u32,
        after_id: Option<ResourceActivityId>,
        kind: Option<ResourceActivityKind>,
        status: Option<ResourceActivityStatus>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "association list limit must be 1..100".into(),
            ));
        }
        Ok(Self {
            limit,
            after_id,
            kind,
            status,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn after_id(self) -> Option<ResourceActivityId> {
        self.after_id
    }
    pub const fn kind(self) -> Option<ResourceActivityKind> {
        self.kind
    }
    pub const fn status(self) -> Option<ResourceActivityStatus> {
        self.status
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceActivityHistoryQuery {
    limit: u32,
    before_revision: Option<ResourceActivityRevision>,
}
impl ResourceActivityHistoryQuery {
    pub fn new(
        limit: u32,
        before_revision: Option<ResourceActivityRevision>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=20).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "association history limit must be 1..20".into(),
            ));
        }
        Ok(Self {
            limit,
            before_revision,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn before_revision(self) -> Option<ResourceActivityRevision> {
        self.before_revision
    }
}
