use super::*;
use crate::resource_activities::ResourceActivityId;
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    procedural_resources::ResourceId,
    resource_hearings::{ResourceHearingId, ResourceHearingOperationId, ResourceHearingRevision},
};

/// Immutable initial capture, including the exact historical validation material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingDetail {
    pub review: ResourceHearingDraft,
    pub material: ResourceHearingMaterial,
    pub revision: ResourceHearingRevision,
    pub recorded_at: OffsetDateTime,
    pub capture_digest: Sha256Digest,
}

/// Durable creation marker. Removing a current association must not remove this
/// evidence or turn a contextual retry into another scheduling operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingOrigin {
    pub case_id: CaseId,
    pub resource_id: ResourceId,
    pub hearing_id: ResourceHearingId,
    pub operation_id: ResourceHearingOperationId,
    pub association_id: ResourceActivityId,
    pub submission_digest: Sha256Digest,
    pub capture_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingCreation {
    pub hearing: ResourceHearingDetail,
    pub origin: ResourceHearingOrigin,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceHearingPreparation {
    Ready(Box<ResourceHearingMaterial>),
    Replay(Box<ResourceHearingCreation>),
}
