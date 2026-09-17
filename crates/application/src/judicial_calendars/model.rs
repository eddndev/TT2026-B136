use super::*;
use domain::{clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JudicialCalendarAction {
    Publish,
    Replace,
    Retire,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JudicialCalendarChange {
    Publish {
        values: JudicialCalendarValues,
    },
    Replace {
        expected_revision: JudicialCalendarRevision,
        values: JudicialCalendarValues,
        reason: JudicialCalendarReason,
    },
    Retire {
        expected_revision: JudicialCalendarRevision,
        reason: JudicialCalendarReason,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarCommand {
    pub operation_id: JudicialCalendarOperationId,
    pub calendar_id: JudicialCalendarId,
    pub change: JudicialCalendarChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarActorSnapshot {
    pub id: UserId,
    pub email: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarReceipt {
    pub operation_id: JudicialCalendarOperationId,
    pub action: JudicialCalendarAction,
    pub expected_revision: u32,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarDetail {
    pub id: JudicialCalendarId,
    pub revision: JudicialCalendarRevision,
    pub values: JudicialCalendarValues,
    pub values_digest: Sha256Digest,
    pub status: JudicialCalendarStatus,
    pub reason: Option<JudicialCalendarReason>,
    pub receipt: JudicialCalendarReceipt,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: JudicialCalendarActorSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarOverview {
    pub id: JudicialCalendarId,
    pub revision: JudicialCalendarRevision,
    pub status: JudicialCalendarStatus,
    pub scope: JudicialCalendarScope,
    pub coverage: JudicialCalendarCoverage,
    pub values_digest: Sha256Digest,
    pub has_unresolved: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarPage {
    pub calendars: Vec<JudicialCalendarOverview>,
    pub has_more: bool,
    pub next_after_id: Option<JudicialCalendarId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarHistoryEntry {
    pub id: JudicialCalendarId,
    pub revision: JudicialCalendarRevision,
    pub status: JudicialCalendarStatus,
    pub values_digest: Sha256Digest,
    pub reason: Option<JudicialCalendarReason>,
    pub receipt: JudicialCalendarReceipt,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: JudicialCalendarActorSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarHistoryPage {
    pub revisions: Vec<JudicialCalendarHistoryEntry>,
    pub has_more: bool,
    pub next_before_revision: Option<JudicialCalendarRevision>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarDraft {
    pub actor: UserId,
    pub command: JudicialCalendarCommand,
    pub result_revision: JudicialCalendarRevision,
    pub values: JudicialCalendarValues,
    pub values_digest: Sha256Digest,
    pub initial_scope: JudicialCalendarScope,
    pub submission_digest: Sha256Digest,
}
