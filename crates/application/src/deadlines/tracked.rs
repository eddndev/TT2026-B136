use super::*;
use crate::{
    cases::CurrentCaseAdministration,
    deadline_reevaluation::{
        encode_tracked_submission, Observations, PredecessorReceipt, TechnicalCause, TrackedAction,
        TrackedSubmission,
    },
    deadline_tracking::{
        operational_due_at, DeadlineReviewState, TrackingPolicies, TrackingReview,
    },
    ApplicationError,
};
use domain::crypto::Sha256Digest;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineReceiptVersion {
    Legacy,
    Tracked(DeadlineTrackedReceipt),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineTrackedReceipt {
    pub observations_digest: Sha256Digest,
    pub predecessor: Option<PredecessorReceipt>,
    pub cause: Option<TechnicalCause>,
}
/// New observations never replace the single retained historical calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineTrackingCapture {
    pub policies: TrackingPolicies,
    pub review: TrackingReview,
    pub observations: Observations,
    pub administration: CurrentCaseAdministration,
}
impl DeadlineDetail {
    pub fn review_state(&self) -> DeadlineReviewState {
        self.tracking
            .as_ref()
            .map_or(DeadlineReviewState::LegacyUndeclared, |value| {
                value.review.state()
            })
    }
    pub fn operational_due_at(&self) -> Option<OffsetDateTime> {
        operational_due_at(
            self.calculation.result.due_at(),
            self.status,
            self.review_state(),
        )
    }
}

/// Dispatch by stored version. A corrupt V2 record is never retried as V1.
pub fn deadline_record_submission_bytes(
    detail: &DeadlineDetail,
) -> Result<Vec<u8>, ApplicationError> {
    submission_bytes(
        &detail.recorded_by,
        detail.case_id,
        detail.id,
        &detail.receipt,
        detail.reason.as_ref(),
    )
}
pub(super) fn submission_bytes(
    author: &DeadlineActorSnapshot,
    case_id: domain::cases::CaseId,
    id: DeadlineId,
    receipt: &DeadlineReceipt,
    reason: Option<&domain::procedural_facts::FactText>,
) -> Result<Vec<u8>, ApplicationError> {
    match &receipt.version {
        DeadlineReceiptVersion::Legacy => {
            let actor = author
                .user_id()
                .ok_or_else(|| inconsistent("legacy author must be a user"))?;
            if receipt.action == DeadlineAction::Reevaluate {
                return Err(inconsistent("legacy receipt cannot reevaluate"));
            }
            Ok(canonical::receipt_submission_bytes(
                actor, case_id, id, receipt, reason,
            ))
        }
        DeadlineReceiptVersion::Tracked(metadata) => {
            let action = match receipt.action {
                DeadlineAction::Register => TrackedAction::Register,
                DeadlineAction::Correct => TrackedAction::Correct,
                DeadlineAction::SetAttention => TrackedAction::SetAttention,
                DeadlineAction::Retire => TrackedAction::Retire,
                DeadlineAction::Reevaluate => TrackedAction::Reevaluate,
            };
            encode_tracked_submission(&TrackedSubmission {
                case_id,
                deadline_id: id,
                operation_id: receipt.operation_id,
                action,
                expected_revision: receipt.expected_revision,
                review_digest: receipt.review_digest,
                observations_digest: metadata.observations_digest,
                predecessor: metadata.predecessor,
                author: author.clone(),
                reason: reason.map(|value| value.as_str().to_owned()),
                cause: metadata.cause,
            })
            .map_err(|error| inconsistent(&error.to_string()))
        }
    }
}

/// Check linkage shared by complete records and lightweight history projections.
pub(super) fn predecessor_matches(
    previous: &DeadlineReceipt,
    next: &DeadlineReceipt,
) -> Result<(), ApplicationError> {
    match (&previous.version, &next.version) {
        (DeadlineReceiptVersion::Legacy, DeadlineReceiptVersion::Legacy) => Ok(()),
        (DeadlineReceiptVersion::Tracked(_), DeadlineReceiptVersion::Legacy) => Err(inconsistent(
            "tracked history cannot return to legacy receipts",
        )),
        (_, DeadlineReceiptVersion::Tracked(metadata)) => {
            if metadata.predecessor
                != Some(PredecessorReceipt {
                    submission_digest: previous.submission_digest,
                    capture_digest: previous.capture_digest,
                })
            {
                return Err(inconsistent("tracked predecessor commitments differ"));
            }
            Ok(())
        }
    }
}
