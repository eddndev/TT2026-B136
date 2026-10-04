use super::{capture_validation::*, *};
use crate::{identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
    hearings::HearingStatus,
};

/// Checked historical material, not proof of current access, admission or persistence.
#[derive(Debug)]
pub struct CheckedPrecautionaryHearingReview {
    review: PrecautionaryHearingReview,
    earliest_capture: OffsetDateTime,
}

impl CheckedPrecautionaryHearingReview {
    pub fn review(&self) -> &PrecautionaryHearingReview {
        &self.review
    }

    pub fn into_capture(
        self,
        hasher: &dyn DocumentHasher,
        recorded_at: OffsetDateTime,
    ) -> Result<PrecautionaryHearingCapture, ApplicationError> {
        if recorded_at < self.earliest_capture {
            return Err(invalid("capture predates its checked material"));
        }
        let mut capture = PrecautionaryHearingCapture {
            review: self.review,
            recorded_at,
            capture_digest: Sha256Digest::from_array([0; 32]),
        };
        capture.capture_digest = hasher.hash_bytes(&precautionary_hearing_capture_bytes(&capture)?);
        precautionary_hearing_receipt_matches(hasher, &capture)?;
        Ok(capture)
    }
}

/// Constructs an imposition capture review from supplied historical material.
/// This neither authenticates the actor nor admits a document or reserves an operation.
pub fn prepare_precautionary_hearing_capture(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: PrecautionaryHearingCommand,
    observed_context: PrecautionaryContext,
    mut sources: PrecautionaryHearingSources,
    predecessor: Option<&PrecautionaryHearingCapture>,
) -> Result<CheckedPrecautionaryHearingReview, ApplicationError> {
    if (command.action() == PrecautionaryHearingAction::Schedule) != predecessor.is_none() {
        return Err(invalid("instruction predecessor presence differs"));
    }
    if let Some(previous) = predecessor {
        precautionary_hearing_receipt_matches(hasher, previous)?;
    }
    if sources.participants.len() > 32 {
        return Err(invalid("too many participant sources"));
    }
    sources
        .participants
        .sort_by_key(|source| source.id().as_uuid());
    let (resolved_values, scheduling_context, status) = match &command.change {
        PrecautionaryHearingChange::Schedule { values, .. }
        | PrecautionaryHearingChange::Replace { values, .. } => (
            values.clone(),
            observed_context.clone(),
            HearingStatus::Scheduled,
        ),
        PrecautionaryHearingChange::Cancel { .. } => {
            let prior = &predecessor
                .ok_or_else(|| invalid("missing cancellation predecessor"))?
                .review;
            (
                prior.resolved_values.clone(),
                prior.scheduling_context.clone(),
                HearingStatus::Cancelled,
            )
        }
    };
    let participants = resolve_precautionary_participants(
        hasher,
        case_id,
        &resolved_values,
        &sources.participants,
    )?;
    let submission_digest = hasher.hash_bytes(&precautionary_hearing_submission_bytes(
        actor,
        case_id,
        &command,
        &resolved_values,
    )?);
    let mut review = PrecautionaryHearingReview {
        case_id,
        actor: actor.clone(),
        result_revision: command.result_revision()?,
        command,
        resolved_values,
        status,
        scheduling_context,
        observed_context,
        sources,
        participants,
        submission_digest,
        review_digest: Sha256Digest::from_array([0; 32]),
    };
    review.review_digest = hasher.hash_bytes(&precautionary_hearing_review_bytes(&review)?);
    validate_review(hasher, &review)?;
    let mut earliest_capture = latest_source_time(&review)?;
    if let Some(previous) = predecessor {
        transition(previous, &review)?;
        earliest_capture = earliest_capture.max(previous.recorded_at);
    }
    Ok(CheckedPrecautionaryHearingReview {
        review,
        earliest_capture,
    })
}
