//! Validate adjacent captured revisions without recomputing historical arithmetic.
mod administration;
mod calendar;
mod preservation;
mod reevaluation;

use super::*;
use crate::{deadline_tracking::DeadlineReviewState, ApplicationError};
use domain::crypto::DocumentHasher;

/// Verify immutable linkage and action-specific preservation. The repository must
/// also resolve exact observation evidence, authenticate the event and technical
/// service, and enforce operation uniqueness across the complete history.
/// Calendar results are produced by the technical preparer, never replayed here.
pub fn deadline_successor_matches(
    hasher: &dyn DocumentHasher,
    previous: &DeadlineDetail,
    next: &DeadlineDetail,
) -> Result<(), ApplicationError> {
    deadline_receipt_matches(hasher, previous)?;
    deadline_receipt_matches(hasher, next)?;
    if previous.id != next.id || previous.case_id != next.case_id {
        return Err(inconsistent(
            "successor belongs to another deadline or case",
        ));
    }
    if previous.status == DeadlineStatus::Retired {
        return Err(inconsistent("retired deadline cannot have a successor"));
    }
    if previous.revision.get().checked_add(1) != Some(next.revision.get())
        || next.receipt.expected_revision != previous.revision.get()
        || next.receipt.operation_id == previous.receipt.operation_id
    {
        return Err(inconsistent("successor revision or operation differs"));
    }
    tracked::predecessor_matches(&previous.receipt, &next.receipt)?;
    administration::validate(hasher, previous, next)?;
    match next.receipt.action {
        DeadlineAction::Register => Err(inconsistent("registration cannot have a predecessor")),
        DeadlineAction::Correct => {
            if !preservation::same_attention(previous, next) {
                return Err(inconsistent("correction changed the attention declaration"));
            }
            Ok(())
        }
        DeadlineAction::SetAttention | DeadlineAction::Retire => {
            manual_preservation(hasher, previous, next)
        }
        DeadlineAction::Reevaluate => reevaluation::validate(hasher, previous, next),
    }
}

fn manual_preservation(
    hasher: &dyn DocumentHasher,
    previous: &DeadlineDetail,
    next: &DeadlineDetail,
) -> Result<(), ApplicationError> {
    let mut compared = next.clone();
    if next.receipt.action == DeadlineAction::SetAttention {
        compared.attention = previous.attention.clone();
    }
    compared.status = previous.status;
    if !preservation::same_body(hasher, previous, &compared)? {
        return Err(inconsistent(
            "manual followup changed captured deadline evidence",
        ));
    }
    match (&previous.tracking, &next.tracking) {
        (None, None) => Ok(()),
        (None, Some(tracking)) => {
            if tracking.review.state() != DeadlineReviewState::LegacyUndeclared
                || !reevaluation::undeclared(tracking)
            {
                return Err(inconsistent(
                    "manual legacy followup declared tracking policies",
                ));
            }
            Ok(())
        }
        (Some(_), Some(_)) => {
            if deadline_capture_bytes(hasher, previous)?
                != deadline_capture_bytes(hasher, &compared)?
            {
                return Err(inconsistent("manual followup changed tracked evidence"));
            }
            Ok(())
        }
        (Some(_), None) => Err(inconsistent("manual followup discarded tracking evidence")),
    }
}
