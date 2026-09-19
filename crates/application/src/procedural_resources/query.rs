use super::*;
use crate::ApplicationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceQuery {
    limit: u32,
    after_id: Option<ResourceId>,
    kind: Option<ResourceKind>,
    status: Option<ResourceStatus>,
}
impl ResourceQuery {
    pub fn new(
        limit: u32,
        after_id: Option<ResourceId>,
        kind: Option<ResourceKind>,
        status: Option<ResourceStatus>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "resource list limit must be 1..100".into(),
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
    pub const fn after_id(self) -> Option<ResourceId> {
        self.after_id
    }
    pub const fn kind(self) -> Option<ResourceKind> {
        self.kind
    }
    pub const fn status(self) -> Option<ResourceStatus> {
        self.status
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceHistoryQuery {
    limit: u32,
    before_revision: Option<ResourceRevision>,
}
impl ResourceHistoryQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        if !(1..=20).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "resource history limit must be 1..20".into(),
            ));
        }
        Ok(Self {
            limit,
            before_revision: before_revision.map(ResourceRevision::new).transpose()?,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn before_revision(self) -> Option<ResourceRevision> {
        self.before_revision
    }
}
