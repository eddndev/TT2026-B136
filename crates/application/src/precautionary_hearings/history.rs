use super::{capture_validation::invalid, *};
use crate::ApplicationError;
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    precautionary_hearings::{
        PrecautionaryHearingId, PrecautionaryHearingOperationId, PrecautionaryHearingRevision,
    },
};

/// Exact initial capture identity. Its existence in durable storage is a store check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingOrigin {
    pub case_id: CaseId,
    pub hearing_id: PrecautionaryHearingId,
    pub operation_id: PrecautionaryHearingOperationId,
    pub revision: PrecautionaryHearingRevision,
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
    pub capture_digest: Sha256Digest,
}

/// Extracts origin metadata only from a valid initial scheduling capture.
pub fn precautionary_hearing_origin(
    hasher: &dyn DocumentHasher,
    capture: &PrecautionaryHearingCapture,
) -> Result<PrecautionaryHearingOrigin, ApplicationError> {
    precautionary_hearing_origin_with_measure_history(
        hasher,
        capture,
        &crate::precautionary_measures::MeasureHistoryEvidence { groups: vec![] },
    )
}

pub(super) fn origin_metadata(
    capture: &PrecautionaryHearingCapture,
) -> Result<PrecautionaryHearingOrigin, ApplicationError> {
    let review = &capture.review;
    if review.command.action() != PrecautionaryHearingAction::Schedule
        || review.result_revision != PrecautionaryHearingRevision::initial()
    {
        return Err(invalid("origin requires the initial scheduling capture"));
    }
    Ok(PrecautionaryHearingOrigin {
        case_id: review.case_id,
        hearing_id: review.command.hearing_id,
        operation_id: review.command.operation_id,
        revision: review.result_revision,
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
        capture_digest: capture.capture_digest,
    })
}

/// Verifies the supplied ascending chain from revision one against its exact origin.
/// The store must separately establish that the supplied last capture is its current
/// head; this function cannot detect an omitted valid suffix or prove current access.
pub fn precautionary_hearing_history_matches(
    hasher: &dyn DocumentHasher,
    captures: &[PrecautionaryHearingCapture],
    origin: &PrecautionaryHearingOrigin,
) -> Result<(), ApplicationError> {
    precautionary_hearing_history_with_measure_history_matches(
        hasher,
        captures,
        origin,
        &crate::precautionary_measures::MeasureHistoryEvidence { groups: vec![] },
    )
}
