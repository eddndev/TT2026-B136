use super::*;
use crate::ApplicationError;

impl HearingAction {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Schedule => 0,
            Self::Replace => 1,
            Self::Cancel => 2,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Replace => "replace",
            Self::Cancel => "cancel",
        }
    }
}

impl HearingCommand {
    pub const fn action(&self) -> HearingAction {
        match self.change {
            HearingChange::Schedule { .. } => HearingAction::Schedule,
            HearingChange::Replace { .. } => HearingAction::Replace,
            HearingChange::Cancel { .. } => HearingAction::Cancel,
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self.change {
            HearingChange::Schedule { .. } => 0,
            HearingChange::Replace {
                expected_revision, ..
            }
            | HearingChange::Cancel {
                expected_revision, ..
            } => expected_revision.get(),
        }
    }
    pub const fn expected_context(&self) -> Option<HearingContextExpectation> {
        match self.change {
            HearingChange::Schedule { context, .. } | HearingChange::Replace { context, .. } => {
                Some(context)
            }
            HearingChange::Cancel { .. } => None,
        }
    }
    pub const fn reason(&self) -> Option<&HearingNote> {
        match &self.change {
            HearingChange::Schedule { .. } => None,
            HearingChange::Replace { reason, .. } | HearingChange::Cancel { reason, .. } => {
                Some(reason)
            }
        }
    }
    pub fn result_revision(&self) -> Result<HearingRevision, ApplicationError> {
        match self.change {
            HearingChange::Schedule { .. } => Ok(HearingRevision::initial()),
            HearingChange::Replace {
                expected_revision, ..
            }
            | HearingChange::Cancel {
                expected_revision, ..
            } => expected_revision
                .next()
                .ok_or_else(|| HearingError::RevisionExhausted.into()),
        }
    }
}
