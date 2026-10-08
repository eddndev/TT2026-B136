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
        let mut records = Vec::with_capacity(2);
        for result in std::iter::once(&review.result).chain(review.replacement.iter()) {
            let mut row = MeasureAdministrativeRecordCapture {
                case_id: review.case_id,
                operation_id: review.command.operation_id,
                result: result.clone(),
                actor: review.actor.clone(),
                context: review.context.clone(),
                support: review.support.clone(),
                review_digest: review.review_digest,
                recorded_at,
                capture_digest: empty,
            };
            row.capture_digest = hasher.hash_bytes(&measure_administrative_record_bytes(&row)?);
            records.push(row);
        }
        let replacement_link = records.get(1).map(|replacement| {
            let marked = &records[0];
            MeasureAdministrativeReplacementLink {
                entered_in_error: domain::precautionary_hearings::PrecautionaryMeasureRef::new(
                    marked.result.id,
                    marked.result.revision,
                    marked.capture_digest,
                ),
                replacement: domain::precautionary_hearings::PrecautionaryMeasureRef::new(
                    replacement.result.id,
                    replacement.result.revision,
                    replacement.capture_digest,
                ),
            }
        });
        records.sort_by_key(|row| (row.result.id.as_uuid(), row.result.revision.get()));
        let mut capture = MeasureAdministrativeCapture {
            review,
            records,
            replacement_link,
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
    matches_view(
        hasher,
        capture,
        super::record_index::HistoryView {
            judicial: history,
            administrative: &[],
            decisions: &[],
        },
    )
}

pub fn measure_administrative_capture_with_history_matches(
    hasher: &dyn DocumentHasher,
    capture: &MeasureAdministrativeCapture,
    history: &MeasureRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    matches_view(hasher, capture, history.into())
}

fn matches_view(
    hasher: &dyn DocumentHasher,
    capture: &MeasureAdministrativeCapture,
    history: super::record_index::HistoryView<'_>,
) -> Result<(), ApplicationError> {
    super::record_bounds::administrative_shape(capture)?;
    super::record_bounds::limits(
        history,
        1,
        super::record_bounds::candidate_rows(&capture.review.command),
    )?;
    let review = &capture.review;
    let expected = super::preparation::prepare_with_view_with_subject(
        hasher,
        &review.actor,
        review.case_id,
        review.command.clone(),
        review.context.clone(),
        history,
        review
            .replacement
            .as_ref()
            .map(|result| result.sources.subject.clone()),
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
    Ok(origin(capture))
}

pub fn measure_administrative_origin_with_history(
    hasher: &dyn DocumentHasher,
    capture: &MeasureAdministrativeCapture,
    history: &MeasureRecordHistoryEvidence,
) -> Result<MeasureAdministrativeOrigin, ApplicationError> {
    measure_administrative_capture_with_history_matches(hasher, capture, history)?;
    Ok(origin(capture))
}

pub(super) fn origin(capture: &MeasureAdministrativeCapture) -> MeasureAdministrativeOrigin {
    MeasureAdministrativeOrigin {
        case_id: capture.review.case_id,
        operation_id: capture.review.command.operation_id,
        submission_digest: capture.review.submission_digest,
        review_digest: capture.review.review_digest,
        capture_digest: capture.capture_digest,
    }
}

pub fn measure_administrative_capture_with_decision_history_matches(
    hasher: &dyn DocumentHasher,
    capture: &MeasureAdministrativeCapture,
    evidence: &crate::precautionary_measures::MeasureDecisionRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    matches_view(hasher, capture, evidence.into())
}
pub fn measure_administrative_origin_with_decision_history(
    hasher: &dyn DocumentHasher,
    capture: &MeasureAdministrativeCapture,
    evidence: &crate::precautionary_measures::MeasureDecisionRecordHistoryEvidence,
) -> Result<MeasureAdministrativeOrigin, ApplicationError> {
    measure_administrative_capture_with_decision_history_matches(hasher, capture, evidence)?;
    Ok(origin(capture))
}
