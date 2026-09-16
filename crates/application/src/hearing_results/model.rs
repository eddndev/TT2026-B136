use super::*;
use crate::case_stages::StageSupportSnapshot;
use crate::cases::{CaseActorSnapshot, CurrentCaseAdministration};
use crate::hearings::{HearingParticipantSnapshot, HearingSchedulingContext};
use domain::{
    case_administration::CaseRevision,
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    hearings::{HearingId, HearingKind, HearingRevision, HearingStatus, HearingTime},
    identity::UserId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HearingResultAction {
    Record,
    Correct,
    Withdraw,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HearingResultChange {
    Record {
        anchor_revision: HearingRevision,
        continuation: Option<HearingResultContinuationRef>,
        values: HearingResultValues,
    },
    Correct {
        expected_revision: HearingResultRevision,
        values: HearingResultValues,
        reason: HearingResultText,
    },
    Withdraw {
        expected_revision: HearingResultRevision,
        reason: HearingResultText,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultCommand {
    pub operation_id: HearingResultOperationId,
    pub hearing_id: HearingId,
    pub result_id: HearingResultId,
    pub change: HearingResultChange,
}
/// Exact immutable programming source; later heads never replace this identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingResultAnchor {
    pub hearing_id: HearingId,
    pub revision: HearingRevision,
    pub values_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingResultContinuation {
    pub hearing_id: HearingId,
    pub result_id: HearingResultId,
    pub revision: HearingResultRevision,
    pub values_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingResultAnchorSnapshot {
    pub reference: HearingResultAnchor,
    pub status: HearingStatus,
    pub kind: HearingKind,
    pub scheduled_at: HearingTime,
    pub scheduling_context: HearingSchedulingContext,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingResultContinuationSnapshot {
    pub reference: HearingResultContinuation,
    pub status: HearingResultStatus,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultReceipt {
    pub operation_id: HearingResultOperationId,
    pub action: HearingResultAction,
    pub expected_revision: u32,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultSnapshot {
    pub case_id: CaseId,
    pub hearing_id: HearingId,
    pub id: HearingResultId,
    pub revision: HearingResultRevision,
    pub values: HearingResultValues,
    pub values_digest: Sha256Digest,
    pub status: HearingResultStatus,
    pub reason: Option<HearingResultText>,
    pub receipt: HearingResultReceipt,
    pub anchor: HearingResultAnchor,
    pub continuation: Option<HearingResultContinuation>,
    pub recorded_administration_revision: CaseRevision,
    pub recorded_administration_digest: Sha256Digest,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: CaseActorSnapshot,
}
/// Participant identity and its exact represented-subject digest when typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultAttendeeSnapshot {
    pub participant: HearingParticipantSnapshot,
    pub subject_digest: Option<Sha256Digest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultDetail {
    pub snapshot: HearingResultSnapshot,
    pub anchor: HearingResultAnchorSnapshot,
    pub continuation: Option<HearingResultContinuationSnapshot>,
    pub attendees: Vec<HearingResultAttendeeSnapshot>,
    pub support: Option<StageSupportSnapshot>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultOverview {
    pub case_id: CaseId,
    pub hearing_id: HearingId,
    pub id: HearingResultId,
    pub revision: HearingResultRevision,
    pub status: HearingResultStatus,
    pub occurrence: HearingResultOccurrence,
    pub extent: HearingResultExtent,
    pub event_time: DeclaredHearingResultTime,
    pub attendee_count: u8,
    pub agreement_count: u8,
    pub anchor_revision: HearingRevision,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultPage {
    pub results: Vec<HearingResultOverview>,
    pub has_more: bool,
    pub next_after_id: Option<HearingResultId>,
}
/// Bounded history entry without the potentially large values and source projections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultHistoryEntry {
    pub case_id: CaseId,
    pub hearing_id: HearingId,
    pub id: HearingResultId,
    pub revision: HearingResultRevision,
    pub values_digest: Sha256Digest,
    pub status: HearingResultStatus,
    pub reason: Option<HearingResultText>,
    pub receipt: HearingResultReceipt,
    pub anchor: HearingResultAnchor,
    pub continuation: Option<HearingResultContinuation>,
    pub recorded_administration_revision: CaseRevision,
    pub recorded_administration_digest: Sha256Digest,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: CaseActorSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultHistoryPage {
    pub revisions: Vec<HearingResultHistoryEntry>,
    pub has_more: bool,
    pub next_before_revision: Option<HearingResultRevision>,
}
/// Stateless preparation; observed administration is informative, never a CAS token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultDraft {
    pub case_id: CaseId,
    pub actor: UserId,
    pub command: HearingResultCommand,
    pub result_revision: HearingResultRevision,
    pub values: HearingResultValues,
    pub values_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
    pub anchor: HearingResultAnchorSnapshot,
    pub continuation: Option<HearingResultContinuationSnapshot>,
    pub observed_administration: CurrentCaseAdministration,
    pub attendees: Vec<HearingResultAttendeeSnapshot>,
    pub support: Option<StageSupportSnapshot>,
}
