use super::{graph::*, inconsistent};
use application::{precautionary_measures::*, ApplicationError};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::*,
};
use postgres::Transaction;

pub(super) fn selections(outcome: &MeasureDecisionOutcome) -> Vec<PrecautionaryMeasureRef> {
    let mut result = Vec::new();
    for effect in outcome.changes().unwrap_or(&[]) {
        match effect {
            MeasureEffect::Impose(_) => {}
            MeasureEffect::Confirm { previous }
            | MeasureEffect::Modify { previous, .. }
            | MeasureEffect::Revoke { previous }
            | MeasureEffect::Cease { previous } => result.push(*previous),
            MeasureEffect::Substitute { predecessors, .. } => {
                result.extend_from_slice(predecessors)
            }
        }
    }
    result.sort_by_key(|r| r.id().as_uuid());
    result
}
pub(super) fn predecessors(
    refs: &[PrecautionaryMeasureRef],
    evidence: &MeasureHistoryEvidence,
) -> Result<Vec<OwnedMeasureMaterial>, ApplicationError> {
    let mut result = Vec::with_capacity(refs.len());
    for reference in refs {
        let mut found = None;
        for group in &evidence.groups {
            for capture in &group.capture.measures {
                if capture.result.id == reference.id()
                    && capture.result.revision == reference.revision()
                {
                    if found.is_some() || capture.capture_digest != reference.digest() {
                        return Err(inconsistent("selected measure ownership differs"));
                    }
                    found = Some(OwnedMeasureMaterial {
                        owner: MeasureGroupRef {
                            operation_id: group.origin.operation_id,
                            decision_id: group.origin.decision_id,
                            group_digest: group.origin.group_digest,
                        },
                        capture: capture.clone(),
                    });
                }
            }
        }
        result.push(found.ok_or_else(|| inconsistent("selected measure has no complete owner"))?);
    }
    result.sort_by_key(|m| m.capture.result.id.as_uuid());
    Ok(result)
}
pub(super) fn operation(
    tx: &mut Transaction<'_>,
    case: CaseId,
    op: MeasureDecisionOperationId,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
    load_precautionary_history(
        tx,
        case,
        &[HistoryRoot::Decision(op)],
        HistoryReserve::default(),
        hasher,
    )?
    .into_operation(op.as_uuid())
}
