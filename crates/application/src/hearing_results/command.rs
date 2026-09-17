use super::*;
use crate::ApplicationError;
impl HearingResultAction {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Record => 0,
            Self::Correct => 1,
            Self::Withdraw => 2,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::Correct => "correct",
            Self::Withdraw => "withdraw",
        }
    }
}
impl HearingResultCommand {
    pub const fn action(&self) -> HearingResultAction {
        match self.change {
            HearingResultChange::Record { .. } => HearingResultAction::Record,
            HearingResultChange::Correct { .. } => HearingResultAction::Correct,
            HearingResultChange::Withdraw { .. } => HearingResultAction::Withdraw,
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self.change {
            HearingResultChange::Record { .. } => 0,
            HearingResultChange::Correct {
                expected_revision, ..
            }
            | HearingResultChange::Withdraw {
                expected_revision, ..
            } => expected_revision.get(),
        }
    }
    pub fn result_revision(&self) -> Result<HearingResultRevision, ApplicationError> {
        match self.change {
            HearingResultChange::Record { .. } => Ok(HearingResultRevision::initial()),
            HearingResultChange::Correct {
                expected_revision, ..
            }
            | HearingResultChange::Withdraw {
                expected_revision, ..
            } => expected_revision
                .next()
                .ok_or_else(|| HearingResultError::RevisionExhausted.into()),
        }
    }
    pub const fn reason(&self) -> Option<&HearingResultText> {
        match &self.change {
            HearingResultChange::Record { .. } => None,
            HearingResultChange::Correct { reason, .. }
            | HearingResultChange::Withdraw { reason, .. } => Some(reason),
        }
    }
}
