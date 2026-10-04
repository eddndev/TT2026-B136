use super::{
    encoding::blob, precautionary_hearing_submission_bytes, PrecautionaryHearingCapture,
    PrecautionaryHearingReview,
};
use crate::ApplicationError;
use domain::{crypto::ArchiveEntry, hearings::HearingStatus};

/// Stable framing of complete reviewed material; consistency requires receipt validation.
pub fn precautionary_hearing_review_bytes(
    review: &PrecautionaryHearingReview,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"PHPR1".to_vec();
    blob(
        &mut bytes,
        &precautionary_hearing_submission_bytes(
            &review.actor,
            review.case_id,
            &review.command,
            &review.resolved_values,
        )?,
    );
    bytes.extend_from_slice(review.submission_digest.as_bytes());
    bytes.extend_from_slice(&review.result_revision.get().to_be_bytes());
    bytes.push(match review.status {
        HearingStatus::Scheduled => 0,
        HearingStatus::Cancelled => 1,
    });
    blob(&mut bytes, &review.scheduling_context.canonical_bytes());
    blob(&mut bytes, &review.observed_context.canonical_bytes());
    ArchiveEntry::new(review.sources.support.name.clone(), Vec::new())?;
    super::source_encoding::sources(&mut bytes, &review.sources)?;
    super::source_encoding::projections(&mut bytes, &review.participants)?;
    Ok(bytes)
}

pub fn precautionary_hearing_capture_bytes(
    capture: &PrecautionaryHearingCapture,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"PHCR1".to_vec();
    blob(
        &mut bytes,
        &precautionary_hearing_review_bytes(&capture.review)?,
    );
    bytes.extend_from_slice(capture.review.review_digest.as_bytes());
    super::context_encoding::timestamp(&mut bytes, capture.recorded_at);
    Ok(bytes)
}
