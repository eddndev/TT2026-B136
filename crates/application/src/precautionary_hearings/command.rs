use crate::ApplicationError;
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    crypto::Sha256Digest,
    hearings::HearingNote,
    precautionary_hearings::{
        PrecautionaryHearingId, PrecautionaryHearingOperationId, PrecautionaryHearingRevision,
        PrecautionaryHearingValues,
    },
};

/// The complete reviewed context is committed separately from its revision counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrecautionaryContextExpectation {
    pub administration_revision: CaseRevision,
    pub stage_revision: CaseStageRevision,
    pub context_digest: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecautionaryHearingAction {
    Schedule,
    Replace,
    Cancel,
}

impl PrecautionaryHearingAction {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrecautionaryHearingChange {
    Schedule {
        context: PrecautionaryContextExpectation,
        values: PrecautionaryHearingValues,
    },
    Replace {
        expected_revision: PrecautionaryHearingRevision,
        expected_capture_digest: Sha256Digest,
        context: PrecautionaryContextExpectation,
        values: PrecautionaryHearingValues,
        reason: HearingNote,
    },
    Cancel {
        expected_revision: PrecautionaryHearingRevision,
        expected_capture_digest: Sha256Digest,
        reason: HearingNote,
    },
}

/// Intent only: existence, current access and exact predecessor require store checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingCommand {
    pub operation_id: PrecautionaryHearingOperationId,
    pub hearing_id: PrecautionaryHearingId,
    pub change: PrecautionaryHearingChange,
}

impl PrecautionaryHearingCommand {
    pub const fn action(&self) -> PrecautionaryHearingAction {
        match self.change {
            PrecautionaryHearingChange::Schedule { .. } => PrecautionaryHearingAction::Schedule,
            PrecautionaryHearingChange::Replace { .. } => PrecautionaryHearingAction::Replace,
            PrecautionaryHearingChange::Cancel { .. } => PrecautionaryHearingAction::Cancel,
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self.change {
            PrecautionaryHearingChange::Schedule { .. } => 0,
            PrecautionaryHearingChange::Replace {
                expected_revision, ..
            }
            | PrecautionaryHearingChange::Cancel {
                expected_revision, ..
            } => expected_revision.get(),
        }
    }
    pub fn result_revision(&self) -> Result<PrecautionaryHearingRevision, ApplicationError> {
        match self.change {
            PrecautionaryHearingChange::Schedule { .. } => {
                Ok(PrecautionaryHearingRevision::initial())
            }
            PrecautionaryHearingChange::Replace {
                expected_revision, ..
            }
            | PrecautionaryHearingChange::Cancel {
                expected_revision, ..
            } => expected_revision.next().ok_or_else(|| {
                ApplicationError::InvalidInput("precautionary revision exhausted".into())
            }),
        }
    }
    pub(super) const fn context(&self) -> Option<PrecautionaryContextExpectation> {
        match self.change {
            PrecautionaryHearingChange::Schedule { context, .. }
            | PrecautionaryHearingChange::Replace { context, .. } => Some(context),
            PrecautionaryHearingChange::Cancel { .. } => None,
        }
    }
    pub(super) const fn predecessor(&self) -> Option<Sha256Digest> {
        match self.change {
            PrecautionaryHearingChange::Schedule { .. } => None,
            PrecautionaryHearingChange::Replace {
                expected_capture_digest,
                ..
            }
            | PrecautionaryHearingChange::Cancel {
                expected_capture_digest,
                ..
            } => Some(expected_capture_digest),
        }
    }
    pub(super) fn reason(&self) -> Option<&HearingNote> {
        match &self.change {
            PrecautionaryHearingChange::Schedule { .. } => None,
            PrecautionaryHearingChange::Replace { reason, .. }
            | PrecautionaryHearingChange::Cancel { reason, .. } => Some(reason),
        }
    }
}
