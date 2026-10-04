use crate::{
    case_stages::StageSupportSnapshot,
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
    procedural_facts::FactParticipantProjection,
    procedural_resources::ResourceDetail,
    resource_activities::{ResourceActCaptureRef, ResourceActivityId, ResourceCaptureRef},
    typed_participants::ParticipantDetail,
};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    procedural_resources::ResourceRevision,
    resource_hearings::{ResourceHearingId, ResourceHearingOperationId, ResourceHearingValues},
};

/// Explicit creation intent; dates and the need for a hearing are never inferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingCommand {
    pub operation_id: ResourceHearingOperationId,
    pub hearing_id: ResourceHearingId,
    pub association_id: ResourceActivityId,
    pub expected_resource_revision: ResourceRevision,
    pub resource: ResourceCaptureRef,
    pub act: Option<ResourceActCaptureRef>,
    pub values: ResourceHearingValues,
}

/// Authorized, server-resolved sources. Participant revisions must be current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingMaterial {
    pub case_id: CaseId,
    pub administration: CurrentCaseAdministration,
    pub resource_head: ResourceDetail,
    pub resource: ResourceDetail,
    pub act: Option<ResourceDetail>,
    pub participants: Vec<ParticipantDetail>,
}

/// Review evidence only: this is not a committed appointment or an operation receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingDraft {
    pub case_id: CaseId,
    pub command: ResourceHearingCommand,
    pub resource: ResourceDetail,
    pub act: Option<ResourceDetail>,
    pub support: StageSupportSnapshot,
    pub participants: Vec<FactParticipantProjection>,
    pub observed_administration: CurrentCaseAdministration,
    pub observed_resource_head: ResourceCaptureRef,
    pub recorded_by: CaseActorSnapshot,
    pub submission_digest: Sha256Digest,
}
