use super::{
    MeasureAdministrativeCapture, MeasureAdministrativeDependencyInventory,
    MeasureAdministrativeOrigin,
};
use crate::{
    documents::DocumentRecord, precautionary_hearings::PrecautionaryContext,
    precautionary_measures::MeasureDecisionRecordHistoryEvidence,
};
use domain::{crypto::Sha256Digest, precautionary_hearings::PrecautionaryMeasureRef};

/// Observed sources and inventory; the store separately proves durable admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeReady {
    pub context: PrecautionaryContext,
    pub support_record: DocumentRecord,
    pub target_head: PrecautionaryMeasureRef,
    pub dependency_inventory: MeasureAdministrativeDependencyInventory,
}

/// Original receipt and exact ancestor closure, excluding this administrative owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeStoredOperation {
    pub capture: MeasureAdministrativeCapture,
    pub origin: MeasureAdministrativeOrigin,
    pub record_history: MeasureDecisionRecordHistoryEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureAdministrativePreparation {
    Ready(Box<MeasureAdministrativeReady>),
    Replay(Box<MeasureAdministrativeStoredOperation>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureAdministrativeConfirmation {
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
}
