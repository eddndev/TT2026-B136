use super::*;
use crate::{
    case_stages::StageSupportSnapshot,
    identity::Principal,
    measure_corrections::{MeasureRecordHistoryEvidence, MeasureRecordRoot, OwnedMeasureRecord},
    precautionary_hearings::PrecautionaryContext,
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
    precautionary_measures::{MeasureDecisionId, MeasureDecisionOperationId, MeasureValues},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionMaterialV2 {
    pub context: PrecautionaryContext,
    pub support: StageSupportSnapshot,
    pub anchor: Option<MeasureDecisionAnchorMaterial>,
    pub predecessors: Vec<OwnedMeasureRecord>,
    pub result_sources: Vec<MeasureResultSources>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedMeasureResultV2 {
    pub id: MeasureId,
    pub revision: MeasureRevision,
    pub record_root: MeasureRecordRoot,
    pub judicial_origin: MeasureOriginIds,
    pub effect_key: MeasureId,
    pub action: MeasureCaptureAction,
    pub previous: Option<PrecautionaryMeasureRef>,
    pub values: MeasureValues,
    pub sources: MeasureSources,
    pub projection: MeasureSourceProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionReviewV2 {
    pub case_id: CaseId,
    pub actor: Principal,
    pub command: MeasureDecisionCommand,
    pub material: MeasureDecisionMaterialV2,
    pub results: Vec<ReviewedMeasureResultV2>,
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureCaptureV2 {
    pub case_id: CaseId,
    pub result: ReviewedMeasureResultV2,
    pub operation_id: MeasureDecisionOperationId,
    pub decision_id: MeasureDecisionId,
    pub decision_digest: Sha256Digest,
    pub actor: Principal,
    pub recorded_at: OffsetDateTime,
    pub capture_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedMeasureMaterialV2 {
    pub owner: MeasureGroupRef,
    pub capture: MeasureCaptureV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionGroupCaptureV2 {
    pub review: MeasureDecisionReviewV2,
    pub decision: MeasureDecisionCapture,
    pub measures: Vec<MeasureCaptureV2>,
    pub substitutions: Vec<MeasureSubstitutionCapture>,
    pub recorded_at: OffsetDateTime,
    pub capture_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureGroupEvidenceV2 {
    pub origin: MeasureGroupOrigin,
    pub capture: MeasureDecisionGroupCaptureV2,
}

/// Flat supplied owners, whose exact dependency closure must be reconstructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionRecordHistoryEvidence {
    pub records: MeasureRecordHistoryEvidence,
    pub decisions: Vec<MeasureGroupEvidenceV2>,
}
