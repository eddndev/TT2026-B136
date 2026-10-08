use super::{PrecautionaryContext, PrecautionaryHearingCommand};
use crate::{
    case_stages::StageSupportSnapshot, identity::Principal,
    procedural_facts::FactParticipantProjection, typed_participants::ParticipantDetail,
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    hearings::HearingStatus,
    precautionary_hearings::{PrecautionaryHearingRevision, PrecautionaryHearingValues},
};

/// Retained historical material. A support snapshot alone does not prove fresh admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingSources {
    pub participants: Vec<ParticipantDetail>,
    pub support: StageSupportSnapshot,
}

/// Flat review evidence, validated on reconstruction before disclosure or use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingReview {
    pub case_id: CaseId,
    pub actor: Principal,
    pub command: PrecautionaryHearingCommand,
    pub resolved_values: PrecautionaryHearingValues,
    pub result_revision: PrecautionaryHearingRevision,
    pub status: HearingStatus,
    pub scheduling_context: PrecautionaryContext,
    pub observed_context: PrecautionaryContext,
    pub sources: PrecautionaryHearingSources,
    pub participants: Vec<FactParticipantProjection>,
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
}

/// Self-contained capture; its predecessor and durable origin are separate evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingCapture {
    pub review: PrecautionaryHearingReview,
    pub recorded_at: OffsetDateTime,
    pub capture_digest: Sha256Digest,
}
