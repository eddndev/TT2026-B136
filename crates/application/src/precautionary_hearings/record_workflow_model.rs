use super::*;
use crate::precautionary_measures::MeasureDecisionRecordHistoryEvidence;

/// Complete original hearing prefix and its exact mixed record dependency closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingRecordHistoryEvidence {
    pub origin: PrecautionaryHearingOrigin,
    pub captures: Vec<PrecautionaryHearingCapture>,
    pub record_history: MeasureDecisionRecordHistoryEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingRecordStoredOperation {
    pub capture: PrecautionaryHearingCapture,
    pub history: PrecautionaryHearingRecordHistoryEvidence,
}

/// Fresh selections are separate from retained history and its earlier target closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingRecordReady {
    pub observed_context: PrecautionaryContext,
    pub history: Option<PrecautionaryHearingRecordHistoryEvidence>,
    pub selected_sources: Option<PrecautionaryHearingSelectedSources>,
    pub record_history: MeasureDecisionRecordHistoryEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrecautionaryHearingRecordPreparation {
    Ready(Box<PrecautionaryHearingRecordReady>),
    Replay(Box<PrecautionaryHearingRecordStoredOperation>),
}
