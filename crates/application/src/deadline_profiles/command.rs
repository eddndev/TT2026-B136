use super::*;
use crate::ApplicationError;
impl DeadlineProfileAction {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Publish => 0,
            Self::Replace => 1,
            Self::Retire => 2,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Publish => "publish",
            Self::Replace => "replace",
            Self::Retire => "retire",
        }
    }
}
impl DeadlineProfileCommand {
    pub const fn action(&self) -> DeadlineProfileAction {
        match self.change {
            DeadlineProfileChange::Publish { .. } => DeadlineProfileAction::Publish,
            DeadlineProfileChange::Replace { .. } => DeadlineProfileAction::Replace,
            DeadlineProfileChange::Retire { .. } => DeadlineProfileAction::Retire,
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self.change {
            DeadlineProfileChange::Publish { .. } => 0,
            DeadlineProfileChange::Replace {
                expected_revision, ..
            }
            | DeadlineProfileChange::Retire {
                expected_revision, ..
            } => expected_revision.get(),
        }
    }
    pub fn result_revision(&self) -> Result<DeadlineProfileRevision, ApplicationError> {
        let next = self
            .expected_revision()
            .checked_add(1)
            .ok_or(DeadlineProfileError::RevisionExhausted)?;
        Ok(DeadlineProfileRevision::new(next)?)
    }
    pub fn reason(&self) -> Option<&domain::procedural_facts::FactText> {
        match &self.change {
            DeadlineProfileChange::Publish { .. } => None,
            DeadlineProfileChange::Replace { reason, .. }
            | DeadlineProfileChange::Retire { reason, .. } => Some(reason),
        }
    }
    pub const fn result_status(&self) -> DeadlineProfileStatus {
        match self.action() {
            DeadlineProfileAction::Retire => DeadlineProfileStatus::Retired,
            _ => DeadlineProfileStatus::Published,
        }
    }
}
