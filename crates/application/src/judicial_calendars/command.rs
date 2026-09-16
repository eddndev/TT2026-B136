use super::*;
use crate::ApplicationError;
impl JudicialCalendarAction {
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
impl JudicialCalendarCommand {
    pub const fn action(&self) -> JudicialCalendarAction {
        match self.change {
            JudicialCalendarChange::Publish { .. } => JudicialCalendarAction::Publish,
            JudicialCalendarChange::Replace { .. } => JudicialCalendarAction::Replace,
            JudicialCalendarChange::Retire { .. } => JudicialCalendarAction::Retire,
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self.change {
            JudicialCalendarChange::Publish { .. } => 0,
            JudicialCalendarChange::Replace {
                expected_revision, ..
            }
            | JudicialCalendarChange::Retire {
                expected_revision, ..
            } => expected_revision.get(),
        }
    }
    pub fn result_revision(&self) -> Result<JudicialCalendarRevision, ApplicationError> {
        let next = self
            .expected_revision()
            .checked_add(1)
            .ok_or(JudicialCalendarError::RevisionExhausted)?;
        Ok(JudicialCalendarRevision::new(next)?)
    }
    pub fn reason(&self) -> Option<&JudicialCalendarReason> {
        match &self.change {
            JudicialCalendarChange::Publish { .. } => None,
            JudicialCalendarChange::Replace { reason, .. }
            | JudicialCalendarChange::Retire { reason, .. } => Some(reason),
        }
    }
    pub const fn result_status(&self) -> JudicialCalendarStatus {
        match self.action() {
            JudicialCalendarAction::Retire => JudicialCalendarStatus::Retired,
            _ => JudicialCalendarStatus::Published,
        }
    }
}
