use super::{wire::invalid, *};
use crate::{precautionary_measures::MeasureHistoryEvidence, ApplicationError};
use domain::{
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
};

impl CheckedMeasureAdministrativeReview {
    pub fn into_capture(
        self,
        hasher: &dyn DocumentHasher,
        recorded_at: OffsetDateTime,
    ) -> Result<MeasureAdministrativeCapture, ApplicationError> {
        if recorded_at.offset() != time::UtcOffset::UTC
            || !(1..=9999).contains(&recorded_at.year())
            || recorded_at < self.earliest_capture
        {
            return Err(invalid(
                "administrative capture predates evidence or uses unsupported UTC",
            ));
        }
        let review = self.review;
        let empty = Sha256Digest::from_array([0; 32]);
        let mut row = MeasureAdministrativeRecordCapture {
            case_id: review.case_id,
            operation_id: review.command.operation_id,
            result: review.result.clone(),
            actor: review.actor.clone(),
            context: review.context.clone(),
            support: review.support.clone(),
            review_digest: review.review_digest,
            recorded_at,
            capture_digest: empty,
        };
        row.capture_digest = hasher.hash_bytes(&measure_administrative_record_bytes(&row)?);
        let mut capture = MeasureAdministrativeCapture {
            review,
            records: vec![row],
            recorded_at,
            capture_digest: empty,
        };
        capture.capture_digest =
            hasher.hash_bytes(&measure_administrative_capture_bytes(&capture)?);
        Ok(capture)
    }
}

/// Verify the complete administrative receipt without rewriting original judicial evidence.
pub fn measure_administrative_capture_matches(
    hasher: &dyn DocumentHasher,
    capture: &MeasureAdministrativeCapture,
    history: &MeasureHistoryEvidence,
) -> Result<(), ApplicationError> {
    if capture.records.len() != 1 {
        return Err(invalid("correction must own exactly one row"));
    }
    let review = &capture.review;
    let expected = prepare_measure_record_correction(
        hasher,
        &review.actor,
        review.case_id,
        review.command.clone(),
        review.context.clone(),
        history,
    )?
    .into_capture(hasher, capture.recorded_at)?;
    if expected != *capture {
        return Err(invalid(
            "administrative receipt differs from complete reconstruction",
        ));
    }
    Ok(())
}

pub fn measure_administrative_origin(
    hasher: &dyn DocumentHasher,
    capture: &MeasureAdministrativeCapture,
    history: &MeasureHistoryEvidence,
) -> Result<MeasureAdministrativeOrigin, ApplicationError> {
    measure_administrative_capture_matches(hasher, capture, history)?;
    Ok(MeasureAdministrativeOrigin {
        case_id: capture.review.case_id,
        operation_id: capture.review.command.operation_id,
        submission_digest: capture.review.submission_digest,
        review_digest: capture.review.review_digest,
        capture_digest: capture.capture_digest,
    })
}
