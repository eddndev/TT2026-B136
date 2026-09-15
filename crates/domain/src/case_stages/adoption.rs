use super::{CaseStage, DeclaredStageTime, StageNote, StageSupportRef};

/// Explicitly declares a known position without fabricating earlier transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageAdoption {
    stage: CaseStage,
    known_at: DeclaredStageTime,
    reason: StageNote,
    support: StageSupportRef,
}

impl StageAdoption {
    pub const fn new(
        stage: CaseStage,
        known_at: DeclaredStageTime,
        reason: StageNote,
        support: StageSupportRef,
    ) -> Self {
        Self {
            stage,
            known_at,
            reason,
            support,
        }
    }

    pub const fn stage(&self) -> CaseStage {
        self.stage
    }
    pub const fn known_at(&self) -> DeclaredStageTime {
        self.known_at
    }
    pub const fn reason(&self) -> &StageNote {
        &self.reason
    }
    pub const fn support(&self) -> StageSupportRef {
        self.support
    }
}
