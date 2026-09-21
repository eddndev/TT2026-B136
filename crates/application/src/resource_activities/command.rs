use super::*;
use crate::ApplicationError;
use domain::procedural_facts::FactText;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceActivityAction {
    Link,
    Unlink,
}
impl ResourceActivityAction {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Link => 0,
            Self::Unlink => 1,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Unlink => "unlink",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceActivityChange {
    Link {
        selection: ResourceActivitySelection,
    },
    Unlink {
        expected_revision: ResourceActivityRevision,
        reason: FactText,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivityCommand {
    pub operation_id: ResourceActivityOperationId,
    pub association_id: ResourceActivityId,
    pub expected_resource_revision: ResourceRevision,
    pub change: ResourceActivityChange,
}
impl ResourceActivityCommand {
    pub const fn action(&self) -> ResourceActivityAction {
        match self.change {
            ResourceActivityChange::Link { .. } => ResourceActivityAction::Link,
            ResourceActivityChange::Unlink { .. } => ResourceActivityAction::Unlink,
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self.change {
            ResourceActivityChange::Link { .. } => 0,
            ResourceActivityChange::Unlink {
                expected_revision, ..
            } => expected_revision.get(),
        }
    }
    pub fn result_revision(&self) -> Result<ResourceActivityRevision, ApplicationError> {
        let value = self.expected_revision().checked_add(1).ok_or_else(|| {
            ApplicationError::InvalidInput("association revision exhausted".into())
        })?;
        Ok(ResourceActivityRevision::new(value)?)
    }
    pub const fn reason(&self) -> Option<&FactText> {
        match &self.change {
            ResourceActivityChange::Link { .. } => None,
            ResourceActivityChange::Unlink { reason, .. } => Some(reason),
        }
    }
}
