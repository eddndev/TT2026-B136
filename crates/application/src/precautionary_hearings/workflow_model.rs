use super::*;
use crate::{
    documents::DocumentRecord, precautionary_measures::MeasureHistoryEvidence,
    typed_participants::ParticipantDetail,
};
use domain::crypto::Sha256Digest;

/// Supplied complete prefix; the authorized store separately establishes durable existence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingHistoryEvidence {
    pub origin: PrecautionaryHearingOrigin,
    pub captures: Vec<PrecautionaryHearingCapture>,
    pub measure_history: MeasureHistoryEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingStoredOperation {
    pub capture: PrecautionaryHearingCapture,
    pub history: PrecautionaryHearingHistoryEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingSelectedSources {
    pub participants: Vec<ParticipantDetail>,
    pub support_record: DocumentRecord,
}

/// Fresh selection and prior history are distinct; cancellation retains prior sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingReady {
    pub observed_context: PrecautionaryContext,
    pub history: Option<PrecautionaryHearingHistoryEvidence>,
    pub selected_sources: Option<PrecautionaryHearingSelectedSources>,
    pub measure_history: MeasureHistoryEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrecautionaryHearingPreparation {
    Ready(Box<PrecautionaryHearingReady>),
    Replay(Box<PrecautionaryHearingStoredOperation>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrecautionaryHearingConfirmation {
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
}
