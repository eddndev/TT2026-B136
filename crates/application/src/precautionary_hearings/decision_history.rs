use super::*;
use crate::{
    identity::Principal, precautionary_measures::MeasureDecisionRecordHistoryEvidence,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};

/// Supplied exact owners do not establish current durable heads or authorization.
pub struct PrecautionaryHearingDecisionPreparationMaterial<'a> {
    pub observed_context: PrecautionaryContext,
    pub sources: PrecautionaryHearingSources,
    pub predecessor: Option<&'a PrecautionaryHearingCapture>,
    pub decision_history: &'a MeasureDecisionRecordHistoryEvidence,
}
pub fn prepare_precautionary_hearing_with_decision_history(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: PrecautionaryHearingCommand,
    material: PrecautionaryHearingDecisionPreparationMaterial<'_>,
) -> Result<CheckedPrecautionaryHearingReview, ApplicationError> {
    super::record_evidence::prepare_with_view(
        hasher,
        actor,
        case_id,
        command,
        super::record_evidence::PreparationView {
            observed_context: material.observed_context,
            sources: material.sources,
            predecessor: material.predecessor,
            history: material.decision_history.into(),
        },
    )
}
pub fn precautionary_hearing_receipt_with_decision_history_matches(
    hasher: &dyn DocumentHasher,
    capture: &PrecautionaryHearingCapture,
    evidence: &MeasureDecisionRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    super::record_evidence::check_captures(hasher, &[capture], evidence.into())
}
pub fn precautionary_hearing_transition_with_decision_history_matches(
    hasher: &dyn DocumentHasher,
    previous: &PrecautionaryHearingCapture,
    next: &PrecautionaryHearingCapture,
    evidence: &MeasureDecisionRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    super::record_history::transition_with_view(hasher, previous, next, evidence.into())
}
pub fn precautionary_hearing_origin_with_decision_history(
    hasher: &dyn DocumentHasher,
    capture: &PrecautionaryHearingCapture,
    evidence: &MeasureDecisionRecordHistoryEvidence,
) -> Result<PrecautionaryHearingOrigin, ApplicationError> {
    precautionary_hearing_receipt_with_decision_history_matches(hasher, capture, evidence)?;
    super::history::origin_metadata(capture)
}
pub fn precautionary_hearing_history_with_decision_history_matches(
    hasher: &dyn DocumentHasher,
    captures: &[PrecautionaryHearingCapture],
    origin: &PrecautionaryHearingOrigin,
    evidence: &MeasureDecisionRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    super::record_history::history_with_view(hasher, captures, origin, evidence.into())
}
