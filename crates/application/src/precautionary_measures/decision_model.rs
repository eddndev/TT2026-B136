use super::{MeasureSourceProjection, MeasureSources};
use crate::{
    case_stages::StageSupportSnapshot,
    hearings::HearingDetail,
    identity::Principal,
    precautionary_hearings::{
        PrecautionaryContext, PrecautionaryContextExpectation, PrecautionaryHearingCapture,
    },
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    hearings::{HearingId, HearingRevision},
    precautionary_hearings::{
        MeasureId, MeasureRevision, PrecautionaryHearingId, PrecautionaryHearingRevision,
        PrecautionaryMeasureRef,
    },
    precautionary_measures::{
        MeasureDecisionId, MeasureDecisionOperationId, MeasureDecisionOutcome,
        MeasureDecisionValues, MeasureValues,
    },
};

/// Exact declared link. Admission requires the corresponding complete family proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureDecisionAnchorRef {
    Initial {
        hearing_id: HearingId,
        revision: HearingRevision,
        values_digest: Sha256Digest,
        submission_digest: Sha256Digest,
    },
    Precautionary {
        hearing_id: PrecautionaryHearingId,
        revision: PrecautionaryHearingRevision,
        capture_digest: Sha256Digest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureDecisionAnchorMaterial {
    Initial(Box<HearingDetail>),
    Precautionary(Box<PrecautionaryHearingCapture>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionCommand {
    pub operation_id: MeasureDecisionOperationId,
    pub decision_id: MeasureDecisionId,
    pub context: PrecautionaryContextExpectation,
    pub values: MeasureDecisionValues,
    pub anchor: Option<MeasureDecisionAnchorRef>,
    pub outcome: MeasureDecisionOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureOriginIds {
    pub decision_id: MeasureDecisionId,
    pub operation_id: MeasureDecisionOperationId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureResultSources {
    pub id: MeasureId,
    pub sources: MeasureSources,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureGroupRef {
    pub operation_id: MeasureDecisionOperationId,
    pub decision_id: MeasureDecisionId,
    pub group_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedMeasureMaterial {
    pub owner: MeasureGroupRef,
    pub capture: MeasureCapture,
}

/// Supplied history, not proof of admission, ownership or current permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionMaterial {
    pub context: PrecautionaryContext,
    pub support: StageSupportSnapshot,
    pub anchor: Option<MeasureDecisionAnchorMaterial>,
    pub predecessors: Vec<OwnedMeasureMaterial>,
    pub result_sources: Vec<MeasureResultSources>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasureCaptureAction {
    Impose,
    Confirm,
    Modify,
    Revoke,
    Cease,
    SubstituteOut,
    SubstituteIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedMeasureResult {
    pub id: MeasureId,
    pub revision: MeasureRevision,
    pub origin: MeasureOriginIds,
    pub effect_key: MeasureId,
    pub action: MeasureCaptureAction,
    pub previous: Option<PrecautionaryMeasureRef>,
    pub values: MeasureValues,
    pub sources: MeasureSources,
    pub projection: MeasureSourceProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionReview {
    pub case_id: CaseId,
    pub actor: Principal,
    pub command: MeasureDecisionCommand,
    pub material: MeasureDecisionMaterial,
    pub results: Vec<ReviewedMeasureResult>,
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionCapture {
    pub case_id: CaseId,
    pub operation_id: MeasureDecisionOperationId,
    pub decision_id: MeasureDecisionId,
    pub actor: Principal,
    pub context: PrecautionaryContext,
    pub values: MeasureDecisionValues,
    pub support: StageSupportSnapshot,
    pub anchor: Option<MeasureDecisionAnchorMaterial>,
    pub recorded_at: OffsetDateTime,
    pub capture_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureCapture {
    pub case_id: CaseId,
    pub result: ReviewedMeasureResult,
    pub operation_id: MeasureDecisionOperationId,
    pub decision_id: MeasureDecisionId,
    pub decision_digest: Sha256Digest,
    pub actor: Principal,
    pub recorded_at: OffsetDateTime,
    pub capture_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureSubstitutionPredecessor {
    pub previous: PrecautionaryMeasureRef,
    pub result: PrecautionaryMeasureRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureSubstitutionCapture {
    pub effect_key: MeasureId,
    pub predecessors: Vec<MeasureSubstitutionPredecessor>,
    pub successors: Vec<PrecautionaryMeasureRef>,
}

/// Full supplied group; flat consistency alone does not prove durable creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionGroupCapture {
    pub review: MeasureDecisionReview,
    pub decision: MeasureDecisionCapture,
    pub measures: Vec<MeasureCapture>,
    pub substitutions: Vec<MeasureSubstitutionCapture>,
    pub recorded_at: OffsetDateTime,
    pub capture_digest: Sha256Digest,
}
