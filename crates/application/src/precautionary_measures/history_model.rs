use super::*;
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    precautionary_measures::{MeasureDecisionId, MeasureDecisionOperationId},
};

/// Supplied durable-origin fields; their consistency does not prove storage existence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureGroupOrigin {
    pub case_id: CaseId,
    pub operation_id: MeasureDecisionOperationId,
    pub decision_id: MeasureDecisionId,
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
    pub decision_digest: Sha256Digest,
    pub group_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureGroupEvidence {
    pub origin: MeasureGroupOrigin,
    pub capture: MeasureDecisionGroupCapture,
}

/// Exact dependency closure, without substituting current source or measure heads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureHistoryEvidence {
    pub groups: Vec<MeasureGroupEvidence>,
}

#[derive(Debug)]
pub struct CheckedMeasureTargets<'a> {
    pub(super) targets: Vec<OwnedMeasureMaterial>,
    pub(super) groups: Vec<&'a MeasureDecisionGroupCapture>,
}

impl<'a> CheckedMeasureTargets<'a> {
    pub fn targets(&self) -> &[OwnedMeasureMaterial] {
        &self.targets
    }

    pub(crate) fn member(
        &self,
        reference: domain::precautionary_hearings::PrecautionaryMeasureRef,
    ) -> Result<&OwnedMeasureMaterial, crate::ApplicationError> {
        self.targets
            .iter()
            .find(|item| super::effect_resolution::capture_reference(&item.capture) == reference)
            .ok_or_else(|| super::decision_wire::invalid("exact target proof is absent"))
    }

    pub(crate) fn groups(&self) -> &[&'a MeasureDecisionGroupCapture] {
        &self.groups
    }
}

pub(crate) fn origin(group: &MeasureDecisionGroupCapture) -> MeasureGroupOrigin {
    MeasureGroupOrigin {
        case_id: group.review.case_id,
        operation_id: group.review.command.operation_id,
        decision_id: group.review.command.decision_id,
        submission_digest: group.review.submission_digest,
        review_digest: group.review.review_digest,
        decision_digest: group.decision.capture_digest,
        group_digest: group.capture_digest,
    }
}
