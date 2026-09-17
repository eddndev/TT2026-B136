use super::*;
use crate::case_stages::{CurrentCaseStage, StageSupportSnapshot};
use crate::cases::{CaseActorSnapshot, CurrentCaseAdministration};
use crate::participants::ParticipantOverview;
use domain::case_administration::{CaseAdministrativeStatus, CaseRevision, CaseStageRevision};
use domain::case_stages::CaseStage;
use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::crypto::Sha256Digest;
use domain::identity::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingContextExpectation {
    pub case_revision: CaseRevision,
    pub stage_revision: CaseStageRevision,
}

/// Consistent current context; an incomplete case is still readable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingCaseContext {
    pub case_id: CaseId,
    pub administration: CurrentCaseAdministration,
    pub stage: CurrentCaseStage,
}

/// Exact context of scheduling, preserved unchanged by cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingSchedulingContext {
    pub administration_revision: CaseRevision,
    pub administration_digest: Sha256Digest,
    pub stage_revision: CaseStageRevision,
    pub stage: CaseStage,
    /// Initial registration has no separate stage-values digest.
    pub stage_digest: Option<Sha256Digest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HearingAction {
    Schedule,
    Replace,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HearingChange {
    Schedule {
        context: HearingContextExpectation,
        values: HearingValues,
    },
    Replace {
        expected_revision: HearingRevision,
        context: HearingContextExpectation,
        values: HearingValues,
        reason: HearingNote,
    },
    Cancel {
        expected_revision: HearingRevision,
        reason: HearingNote,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingCommand {
    pub operation_id: HearingOperationId,
    pub hearing_id: HearingId,
    pub change: HearingChange,
}

/// Proof of one committed command, distinct from similarity of displayed values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingReceipt {
    pub operation_id: HearingOperationId,
    pub action: HearingAction,
    pub expected_revision: u32,
    pub expected_context: Option<HearingContextExpectation>,
    pub submission_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingParticipantSnapshot {
    pub overview: ParticipantOverview,
    pub values_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingSnapshot {
    pub case_id: CaseId,
    pub id: HearingId,
    pub revision: HearingRevision,
    pub values: HearingValues,
    pub values_digest: Sha256Digest,
    pub status: HearingStatus,
    pub reason: Option<HearingNote>,
    pub receipt: HearingReceipt,
    pub scheduling_context: HearingSchedulingContext,
    pub recorded_administration_revision: CaseRevision,
    pub recorded_administration_digest: Sha256Digest,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: CaseActorSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingDetail {
    pub snapshot: HearingSnapshot,
    pub participants: Vec<HearingParticipantSnapshot>,
    pub support: Option<StageSupportSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingOverview {
    pub case_id: CaseId,
    pub case_title: String,
    pub case_reference: String,
    pub case_status: CaseAdministrativeStatus,
    pub id: HearingId,
    pub revision: HearingRevision,
    pub kind: HearingKind,
    pub scheduled_at: HearingTime,
    pub modality: HearingModality,
    pub status: HearingStatus,
    pub participant_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingPage {
    pub hearings: Vec<HearingOverview>,
    pub has_more: bool,
    pub next_after_id: Option<HearingId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingAgendaPage {
    pub hearings: Vec<HearingOverview>,
    pub has_more: bool,
    pub next_after: Option<HearingAgendaCursor>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingHistoryPage {
    pub revisions: Vec<HearingDetail>,
    pub has_more: bool,
    pub next_before_revision: Option<HearingRevision>,
}

/// Stateless preparation does not reserve a hearing or operation identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingDraft {
    pub command: HearingCommand,
    pub actor: UserId,
    pub submission_digest: Sha256Digest,
    pub result_revision: HearingRevision,
    pub values: HearingValues,
    pub values_digest: Sha256Digest,
}
