use super::*;
use crate::ApplicationError;
use domain::procedural_facts::FactText;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAction {
    Register,
    Correct,
    RecordAct,
    CorrectAct,
    Archive,
    Reactivate,
}
impl ResourceAction {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Register => 0,
            Self::Correct => 1,
            Self::RecordAct => 2,
            Self::CorrectAct => 3,
            Self::Archive => 4,
            Self::Reactivate => 5,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceChange {
    Register {
        values: ResourceValues,
    },
    Correct {
        expected_revision: ResourceRevision,
        values: ResourceValues,
        reason: FactText,
    },
    RecordAct {
        expected_revision: ResourceRevision,
        act_id: ResourceActId,
        values: ResourceActValues,
    },
    CorrectAct {
        expected_revision: ResourceRevision,
        act_id: ResourceActId,
        expected_act_revision: ResourceActRevision,
        values: ResourceActValues,
        reason: FactText,
    },
    Archive {
        expected_revision: ResourceRevision,
        reason: FactText,
    },
    Reactivate {
        expected_revision: ResourceRevision,
        reason: FactText,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceCommand {
    pub operation_id: ResourceOperationId,
    pub resource_id: ResourceId,
    pub change: ResourceChange,
}
impl ResourceCommand {
    pub const fn action(&self) -> ResourceAction {
        match self.change {
            ResourceChange::Register { .. } => ResourceAction::Register,
            ResourceChange::Correct { .. } => ResourceAction::Correct,
            ResourceChange::RecordAct { .. } => ResourceAction::RecordAct,
            ResourceChange::CorrectAct { .. } => ResourceAction::CorrectAct,
            ResourceChange::Archive { .. } => ResourceAction::Archive,
            ResourceChange::Reactivate { .. } => ResourceAction::Reactivate,
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self.change {
            ResourceChange::Register { .. } => 0,
            ResourceChange::Correct {
                expected_revision, ..
            }
            | ResourceChange::RecordAct {
                expected_revision, ..
            }
            | ResourceChange::CorrectAct {
                expected_revision, ..
            }
            | ResourceChange::Archive {
                expected_revision, ..
            }
            | ResourceChange::Reactivate {
                expected_revision, ..
            } => expected_revision.get(),
        }
    }
    pub fn result_revision(&self) -> Result<ResourceRevision, ApplicationError> {
        let next = self
            .expected_revision()
            .checked_add(1)
            .ok_or_else(|| ApplicationError::InvalidInput("resource revision exhausted".into()))?;
        Ok(ResourceRevision::new(next)?)
    }
    pub const fn reason(&self) -> Option<&FactText> {
        match &self.change {
            ResourceChange::Correct { reason, .. }
            | ResourceChange::CorrectAct { reason, .. }
            | ResourceChange::Archive { reason, .. }
            | ResourceChange::Reactivate { reason, .. } => Some(reason),
            _ => None,
        }
    }
}
