use super::{decision_wire::invalid, *};
use crate::ApplicationError;
use domain::{
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
};

impl CheckedMeasureDecisionReview {
    pub fn into_group_capture(
        self,
        hasher: &dyn DocumentHasher,
        recorded_at: OffsetDateTime,
    ) -> Result<MeasureDecisionGroupCapture, ApplicationError> {
        if recorded_at.offset() != time::UtcOffset::UTC
            || !(1..=9999).contains(&recorded_at.year())
            || recorded_at < self.earliest_capture
        {
            return Err(invalid(
                "group capture predates its sources or has unsupported clock",
            ));
        }
        let review = self.review;
        let empty = Sha256Digest::from_array([0; 32]);
        let mut decision = MeasureDecisionCapture {
            case_id: review.case_id,
            operation_id: review.command.operation_id,
            decision_id: review.command.decision_id,
            actor: review.actor.clone(),
            context: review.material.context.clone(),
            values: review.command.values.clone(),
            support: review.material.support.clone(),
            anchor: review.material.anchor.clone(),
            recorded_at,
            capture_digest: empty,
        };
        decision.capture_digest = hasher.hash_bytes(&measure_decision_capture_bytes(&decision)?);
        let mut measures = Vec::with_capacity(review.results.len());
        for result in &review.results {
            let mut capture = MeasureCapture {
                case_id: review.case_id,
                result: result.clone(),
                operation_id: review.command.operation_id,
                decision_id: review.command.decision_id,
                decision_digest: decision.capture_digest,
                actor: review.actor.clone(),
                recorded_at,
                capture_digest: empty,
            };
            capture.capture_digest = hasher.hash_bytes(&measure_capture_bytes(&capture)?);
            measures.push(capture);
        }
        let substitutions = super::effect_resolution::substitutions(&review.command, &measures)?;
        let mut group = MeasureDecisionGroupCapture {
            review,
            decision,
            measures,
            substitutions,
            recorded_at,
            capture_digest: empty,
        };
        group.capture_digest = hasher.hash_bytes(&measure_decision_group_bytes(&group)?);
        Ok(group)
    }
}

/// Reconstructs the entire supplied initial group; durable origin and access are separate.
pub fn measure_decision_group_matches(
    hasher: &dyn DocumentHasher,
    group: &MeasureDecisionGroupCapture,
) -> Result<(), ApplicationError> {
    measure_decision_group_with_history_matches(
        hasher,
        group,
        &MeasureHistoryEvidence { groups: vec![] },
    )
}
