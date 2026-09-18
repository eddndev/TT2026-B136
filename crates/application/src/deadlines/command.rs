use super::*;
use crate::ApplicationError;
use domain::procedural_facts::FactText;
impl DeadlineAction {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Register => 0,
            Self::Correct => 1,
            Self::SetAttention => 2,
            Self::Retire => 3,
            Self::Reevaluate => 4,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::Correct => "correct",
            Self::SetAttention => "set_attention",
            Self::Retire => "retire",
            Self::Reevaluate => "reevaluate",
        }
    }
}
impl DeadlineCommand {
    pub const fn action(&self) -> DeadlineAction {
        match self.change {
            DeadlineChange::Register { .. } => DeadlineAction::Register,
            DeadlineChange::Correct { .. } => DeadlineAction::Correct,
            DeadlineChange::SetAttention { .. } => DeadlineAction::SetAttention,
            DeadlineChange::Retire { .. } => DeadlineAction::Retire,
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self.change {
            DeadlineChange::Register { .. } => 0,
            DeadlineChange::Correct {
                expected_revision, ..
            }
            | DeadlineChange::SetAttention {
                expected_revision, ..
            }
            | DeadlineChange::Retire {
                expected_revision, ..
            } => expected_revision.get(),
        }
    }
    pub fn result_revision(&self) -> Result<DeadlineRevision, ApplicationError> {
        DeadlineRevision::new(
            self.expected_revision()
                .checked_add(1)
                .ok_or(DeadlineError::RevisionExhausted)?,
        )
        .map_err(Into::into)
    }
    pub fn reason(&self) -> Option<&FactText> {
        match &self.change {
            DeadlineChange::Register { .. } => None,
            DeadlineChange::Correct { reason, .. }
            | DeadlineChange::SetAttention { reason, .. }
            | DeadlineChange::Retire { reason, .. } => Some(reason),
        }
    }
    pub const fn status(&self) -> DeadlineStatus {
        match self.change {
            DeadlineChange::Retire { .. } => DeadlineStatus::Retired,
            _ => DeadlineStatus::Active,
        }
    }
}
