use super::*;
use crate::{
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
    deadline_currentness::DeadlineCurrent,
    deadlines::DeadlineDetail,
    hearings::HearingDetail,
    procedural_resources::ResourceDetail,
};
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, procedural_facts::FactText,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceActivityTargetDetail {
    Hearing(Box<HearingDetail>),
    Deadline(Box<DeadlineDetail>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivitySources {
    pub resource: ResourceDetail,
    pub act: Option<ResourceDetail>,
    pub target: ResourceActivityTargetDetail,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceActivityRevisionRef {
    pub revision: ResourceActivityRevision,
    pub capture_digest: Sha256Digest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceActivityReceipt {
    pub operation_id: ResourceActivityOperationId,
    pub action: ResourceActivityAction,
    pub expected_revision: u32,
    pub expected_resource_revision: ResourceRevision,
    pub previous: Option<ResourceActivityRevisionRef>,
    pub submission_digest: Sha256Digest,
    pub capture_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivityDetail {
    pub case_id: CaseId,
    pub resource_id: ResourceId,
    pub id: ResourceActivityId,
    pub revision: ResourceActivityRevision,
    pub selection: ResourceActivitySelection,
    pub status: ResourceActivityStatus,
    pub sources: ResourceActivitySources,
    pub reason: Option<FactText>,
    pub receipt: ResourceActivityReceipt,
    pub recorded_by: CaseActorSnapshot,
    pub recorded_at: OffsetDateTime,
    pub recorded_administration: CurrentCaseAdministration,
    pub recorded_resource_head: ResourceCaptureRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivityDraft {
    pub case_id: CaseId,
    pub resource_id: ResourceId,
    pub command: ResourceActivityCommand,
    pub result_revision: ResourceActivityRevision,
    pub selection: ResourceActivitySelection,
    pub status: ResourceActivityStatus,
    pub sources: ResourceActivitySources,
    pub previous: Option<ResourceActivityRevisionRef>,
    pub recorded_by: CaseActorSnapshot,
    pub observed_administration: CurrentCaseAdministration,
    pub observed_resource_head: ResourceCaptureRef,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivityMaterial {
    pub case_id: CaseId,
    pub base: Option<ResourceActivityDetail>,
    pub administration: CurrentCaseAdministration,
    pub resource_head: ResourceDetail,
    pub sources: ResourceActivitySources,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceActivityPreparation {
    Ready(Box<ResourceActivityMaterial>),
    Replay(Box<ResourceActivityDetail>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceActivityCurrentTarget {
    Hearing(Box<HearingDetail>),
    Deadline(Box<DeadlineCurrent>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivityView {
    pub association: ResourceActivityDetail,
    pub checked_at: OffsetDateTime,
    pub current_target: ResourceActivityCurrentTarget,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivityPage {
    pub associations: Vec<ResourceActivityView>,
    pub has_more: bool,
    pub next_after_id: Option<ResourceActivityId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActivityHistoryPage {
    pub revisions: Vec<ResourceActivityDetail>,
    pub has_more: bool,
    pub next_before_revision: Option<ResourceActivityRevision>,
}
