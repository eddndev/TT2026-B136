use super::{AlertCursor, AlertId, AlertOccurrenceId, AlertOperationId};
use crate::{deadlines::DeadlineId, hearings::HearingId};
use domain::{
    alerts::AlertLeadHours, cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest,
    identity::UserId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSubject {
    Hearing { case_id: CaseId, id: HearingId },
    Deadline { case_id: CaseId, id: DeadlineId },
}
impl AlertSubject {
    pub const fn case_id(self) -> CaseId {
        match self {
            Self::Hearing { case_id, .. } | Self::Deadline { case_id, .. } => case_id,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Upcoming {
        lead_hours: AlertLeadHours,
        activity_at: OffsetDateTime,
    },
    OverdueUnattended {
        due_at: OffsetDateTime,
    },
    ReviewRequired,
    DueChangedSoon {
        previous_due_at: OffsetDateTime,
        current_due_at: OffsetDateTime,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertOrigin {
    pub revision: u32,
    pub evidence_digest: Sha256Digest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertResolutionReason {
    Superseded,
    AttentionRecorded,
    TargetRetired,
    CancelledHearing,
    NoLongerEligible,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertState {
    Active,
    Resolved {
        at: OffsetDateTime,
        reason: AlertResolutionReason,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertEmailStatus {
    Disabled,
    Pending,
    Sending,
    Accepted { accepted_at: OffsetDateTime },
    Failed,
    Unknown,
    Cancelled,
}
/// Captured notification evidence is historical; it never certifies a current due.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertRecord {
    pub id: AlertId,
    pub recipient_id: UserId,
    pub occurrence_id: AlertOccurrenceId,
    pub subject: AlertSubject,
    pub subject_title: String,
    pub case_title: String,
    pub case_reference: String,
    pub kind: AlertKind,
    pub origin: AlertOrigin,
    pub trigger_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub read_at: Option<OffsetDateTime>,
    pub state: AlertState,
    pub email: AlertEmailStatus,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertPage {
    pub checked_at: OffsetDateTime,
    pub alerts: Vec<AlertRecord>,
    pub has_more: bool,
    /// May continue after a bounded scan even when no alert was visible.
    pub next_cursor: Option<AlertCursor>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertDetail {
    pub checked_at: OffsetDateTime,
    pub alert: AlertRecord,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertReadCommand {
    pub operation_id: AlertOperationId,
    pub alert_id: AlertId,
}
/// A read receipt never changes the subject's attention or lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertReadReceipt {
    pub operation_id: AlertOperationId,
    pub checked_at: OffsetDateTime,
    pub alert: AlertRecord,
}
