use super::{NotificationId, NotificationValues, ResolutionId};
use crate::{cases::CaseId, DomainError};

/// Stable identity and case binding; construction does not prove persisted existence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionRoot {
    id: ResolutionId,
    case_id: CaseId,
}
impl ResolutionRoot {
    pub const fn new(id: ResolutionId, case_id: CaseId) -> Self {
        Self { id, case_id }
    }
    pub const fn id(self) -> ResolutionId {
        self.id
    }
    pub const fn case_id(self) -> CaseId {
        self.case_id
    }
}

/// One practice keeps its resolution identity; selected revisions may be corrected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationRoot {
    id: NotificationId,
    case_id: CaseId,
    resolution_id: ResolutionId,
}
impl NotificationRoot {
    pub const fn new(id: NotificationId, case_id: CaseId, resolution_id: ResolutionId) -> Self {
        Self {
            id,
            case_id,
            resolution_id,
        }
    }
    pub const fn id(self) -> NotificationId {
        self.id
    }
    pub const fn case_id(self) -> CaseId {
        self.case_id
    }
    pub const fn resolution_id(self) -> ResolutionId {
        self.resolution_id
    }
    /// Checks identity only; existence, case membership, and authorization require ports.
    pub fn validate_values(&self, values: &NotificationValues) -> Result<(), DomainError> {
        if values.resolution().id != self.resolution_id {
            return Err(DomainError::InvalidProceduralFact("resolution_root"));
        }
        Ok(())
    }
}
