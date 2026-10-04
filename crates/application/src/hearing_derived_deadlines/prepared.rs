use super::HearingDerivedDeadlineDraft;
use crate::hearing_results::PreparedHearingResultChange;

/// Only the service constructs this after admitting the exact encrypted support.
pub struct PreparedHearingDerivedDeadline {
    pub(super) draft: HearingDerivedDeadlineDraft,
    pub(super) result: PreparedHearingResultChange,
}

impl PreparedHearingDerivedDeadline {
    pub fn draft(&self) -> &HearingDerivedDeadlineDraft {
        &self.draft
    }

    pub fn result(&self) -> &PreparedHearingResultChange {
        &self.result
    }
}
