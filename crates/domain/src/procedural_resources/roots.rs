use super::{ResourceActId, ResourceId};
use crate::cases::CaseId;

/// Stable case binding; persisted existence and authorization require application ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceRoot {
    id: ResourceId,
    case_id: CaseId,
}
impl ResourceRoot {
    pub const fn new(id: ResourceId, case_id: CaseId) -> Self {
        Self { id, case_id }
    }
    pub const fn id(self) -> ResourceId {
        self.id
    }
    pub const fn case_id(self) -> CaseId {
        self.case_id
    }
}

/// An act belongs to a resource, without creating a new ordinary case stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceActRoot {
    id: ResourceActId,
    resource_id: ResourceId,
}
impl ResourceActRoot {
    pub const fn new(id: ResourceActId, resource_id: ResourceId) -> Self {
        Self { id, resource_id }
    }
    pub const fn id(self) -> ResourceActId {
        self.id
    }
    pub const fn resource_id(self) -> ResourceId {
        self.resource_id
    }
}
