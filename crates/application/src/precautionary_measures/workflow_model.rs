use super::*;
use crate::{documents::DocumentRecord, precautionary_hearings::PrecautionaryContext};
use domain::crypto::Sha256Digest;

/// Fresh exact sources; the store separately proves current access, heads and origins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionReady {
    pub context: PrecautionaryContext,
    pub support_record: DocumentRecord,
    pub anchor: Option<MeasureDecisionAnchorMaterial>,
    pub predecessors: Vec<OwnedMeasureMaterial>,
    pub result_sources: Vec<MeasureResultSources>,
    pub measure_history: MeasureHistoryEvidence,
}

/// Original complete group and its exact ancestor closure, excluding the group itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionStoredOperation {
    pub group: MeasureDecisionGroupCapture,
    pub origin: MeasureGroupOrigin,
    pub measure_history: MeasureHistoryEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureDecisionPreparation {
    Ready(Box<MeasureDecisionReady>),
    Replay(Box<MeasureDecisionStoredOperation>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureDecisionConfirmation {
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
}
