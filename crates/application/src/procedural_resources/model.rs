use super::*;
use crate::{
    case_stages::{CurrentCaseStage, StageSupportSnapshot},
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
    procedural_facts::{FactParticipantProjection, FactResolutionProjection},
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest};

/// Captures are historical evidence; they do not assert current applicability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceSources {
    pub resolution: FactResolutionProjection,
    /// Exact directory revisions, in increasing (id, revision) order.
    pub appellants: Vec<FactParticipantProjection>,
    pub supports: Vec<StageSupportSnapshot>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActCapture {
    pub id: ResourceActId,
    pub revision: ResourceActRevision,
    pub values: ResourceActValues,
    pub supports: Vec<StageSupportSnapshot>,
    /// Binds a correction to its exact earlier resource revision and receipt.
    pub previous: Option<ResourceRevisionRef>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceRevisionRef {
    pub revision: ResourceRevision,
    pub capture_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceReceipt {
    pub operation_id: ResourceOperationId,
    pub action: ResourceAction,
    pub expected_revision: u32,
    pub previous: Option<ResourceRevisionRef>,
    pub values_digest: Sha256Digest,
    pub sources_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
    pub capture_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDetail {
    pub case_id: CaseId,
    pub id: ResourceId,
    pub revision: ResourceRevision,
    pub values: ResourceValues,
    pub status: ResourceStatus,
    pub sources: ResourceSources,
    /// The act recorded or corrected by this revision, not an implicit current act.
    pub act: Option<ResourceActCapture>,
    pub reason: Option<domain::procedural_facts::FactText>,
    pub receipt: ResourceReceipt,
    pub recorded_by: CaseActorSnapshot,
    pub recorded_at: OffsetDateTime,
    pub recorded_administration: CurrentCaseAdministration,
    pub recorded_stage: CurrentCaseStage,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDraft {
    pub case_id: CaseId,
    pub command: ResourceCommand,
    pub result_revision: ResourceRevision,
    pub values: ResourceValues,
    pub status: ResourceStatus,
    pub sources: ResourceSources,
    pub act: Option<ResourceActCapture>,
    pub previous: Option<ResourceRevisionRef>,
    pub recorded_by: CaseActorSnapshot,
    pub observed_administration: CurrentCaseAdministration,
    pub observed_stage: CurrentCaseStage,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourcePage {
    pub resources: Vec<ResourceDetail>,
    pub has_more: bool,
    pub next_after_id: Option<ResourceId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHistoryPage {
    pub revisions: Vec<ResourceDetail>,
    pub has_more: bool,
    pub next_before_revision: Option<ResourceRevision>,
}
