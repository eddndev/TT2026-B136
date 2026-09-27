use crate::{
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
    deadlines::{DeadlineDetail, DeadlineDraft, DeadlineHumanCommand, DeadlinePreparation},
    procedural_resources::ResourceDetail,
    resource_activities::*,
};
use domain::{cases::CaseId, crypto::Sha256Digest};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDeadlineCommand {
    pub association_id: ResourceActivityId,
    pub expected_resource_revision: ResourceRevision,
    pub resource: ResourceCaptureRef,
    pub act: Option<ResourceActCaptureRef>,
    pub deadline: DeadlineHumanCommand,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDeadlineAssociationDraft {
    pub command: ResourceActivityCommand,
    pub resource: ResourceDetail,
    pub act: Option<ResourceDetail>,
    pub recorded_by: CaseActorSnapshot,
    pub observed_administration: CurrentCaseAdministration,
    pub observed_resource_head: ResourceCaptureRef,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDeadlineDraft {
    pub command: ResourceDeadlineCommand,
    pub deadline: DeadlineDraft,
    pub association: ResourceDeadlineAssociationDraft,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDeadlineResult {
    pub deadline: DeadlineDetail,
    pub association: ResourceActivityDetail,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDeadlineMaterial {
    pub case_id: CaseId,
    pub administration: CurrentCaseAdministration,
    pub resource_head: ResourceDetail,
    pub resource: ResourceDetail,
    pub act: Option<ResourceDetail>,
    pub deadline: DeadlinePreparation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceDeadlinePreparation {
    Ready(Box<ResourceDeadlineMaterial>),
    Replay(Box<ResourceDeadlineResult>),
}
