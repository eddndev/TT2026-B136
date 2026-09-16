use super::*;
use crate::{
    case_stages::StageSupportSnapshot,
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
};
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest,
    hearing_results::HearingResultStatus, identity::UserId, participants::DirectoryStatus,
    typed_participants::SubjectRevisionRef,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactFamily {
    Resolution,
    Notification,
}

/// The notification parent belongs to its immutable identity, not its mutable values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactTarget {
    Resolution(ResolutionId),
    Notification {
        id: NotificationId,
        resolution_id: ResolutionId,
    },
}
impl FactTarget {
    pub const fn family(self) -> FactFamily {
        match self {
            Self::Resolution(_) => FactFamily::Resolution,
            Self::Notification { .. } => FactFamily::Notification,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactAction {
    Record,
    Correct,
    Withdraw,
}
impl FactAction {
    pub const fn resulting_status(self) -> FactStatus {
        match self {
            Self::Record | Self::Correct => FactStatus::Recorded,
            Self::Withdraw => FactStatus::Withdrawn,
        }
    }
}

/// Withdrawal retires a declaration; it does not annul an act or its legal effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactStatus {
    Recorded,
    Withdrawn,
}

/// Every digest is resolved by the server, not accepted as proof from a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactResolutionSourceSnapshot {
    pub case_id: CaseId,
    pub reference: FactResolutionRef,
    pub values_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
    pub status: FactStatus,
}

/// The agreement selection is checked inside the exact result revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactHearingSourceSnapshot {
    pub case_id: CaseId,
    pub reference: FactHearingRef,
    pub values_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
    pub status: HearingResultStatus,
}

/// A directory selection retains its own digest and exact bound subject when typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactParticipantSnapshot {
    pub case_id: CaseId,
    pub reference: FactParticipantRef,
    pub values_digest: Sha256Digest,
    pub status: DirectoryStatus,
    pub subject: Option<SubjectRevisionRef>,
}

/// Immediate sources only; historical ancestors do not expand this collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactResolvedSources {
    pub resolution: Option<FactResolutionSourceSnapshot>,
    pub participants: Vec<FactParticipantSnapshot>,
    pub hearing_results: Vec<FactHearingSourceSnapshot>,
}

/// Admission is attached to direct versions; functions and locators remain in values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactSources {
    pub resolved: FactResolvedSources,
    pub views: FactSourceViews,
    pub direct_supports: Vec<StageSupportSnapshot>,
}

/// Receipt digest construction is separate from the value encodings PFRES1/PFNOT1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactReceipt {
    pub operation_id: FactOperationId,
    pub action: FactAction,
    pub expected_revision: u32,
    pub sources_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactRevisionMetadata {
    pub revision: FactRevision,
    pub values_digest: Sha256Digest,
    pub status: FactStatus,
    pub reason: Option<FactText>,
    pub receipt: FactReceipt,
    /// Preserves an unrevised baseline without inventing an administrative revision.
    pub recorded_administration: CurrentCaseAdministration,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: CaseActorSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionSnapshot {
    pub root: ResolutionRoot,
    pub metadata: FactRevisionMetadata,
    pub values: ResolutionValues,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationSnapshot {
    pub root: NotificationRoot,
    pub metadata: FactRevisionMetadata,
    pub values: NotificationValues,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProceduralFactSnapshot {
    Resolution(Box<ResolutionSnapshot>),
    Notification(Box<NotificationSnapshot>),
}
impl ProceduralFactSnapshot {
    pub fn case_id(&self) -> CaseId {
        match self {
            Self::Resolution(v) => v.root.case_id(),
            Self::Notification(v) => v.root.case_id(),
        }
    }
    pub fn target(&self) -> FactTarget {
        match self {
            Self::Resolution(v) => FactTarget::Resolution(v.root.id()),
            Self::Notification(v) => FactTarget::Notification {
                id: v.root.id(),
                resolution_id: v.root.resolution_id(),
            },
        }
    }
    pub fn metadata(&self) -> &FactRevisionMetadata {
        match self {
            Self::Resolution(v) => &v.metadata,
            Self::Notification(v) => &v.metadata,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactDetail {
    pub snapshot: ProceduralFactSnapshot,
    pub sources: FactSources,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionOverview {
    pub root: ResolutionRoot,
    pub revision: FactRevision,
    pub status: FactStatus,
    pub class: FactDeclaration<ResolutionClass>,
    pub issued_at: domain::procedural_time::DeclaredProceduralTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationOverview {
    pub root: NotificationRoot,
    pub revision: FactRevision,
    pub status: FactStatus,
    pub resolution: FactResolutionRef,
    pub outcome: FactDeclaration<NotificationOutcome>,
    pub practiced_at: domain::procedural_time::DeclaredProceduralTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionPage {
    pub resolutions: Vec<ResolutionOverview>,
    pub has_more: bool,
    pub next_after_id: Option<ResolutionId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPage {
    pub notifications: Vec<NotificationOverview>,
    pub has_more: bool,
    pub next_after_id: Option<NotificationId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactHistoryEntry {
    pub case_id: CaseId,
    pub target: FactTarget,
    pub metadata: FactRevisionMetadata,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactHistoryPage {
    pub revisions: Vec<FactHistoryEntry>,
    pub has_more: bool,
    pub next_before_revision: Option<FactRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProceduralFactValues {
    Resolution(Box<ResolutionValues>),
    Notification(Box<NotificationValues>),
}

/// Stateless preparation is informative; it never reserves a root or operation UUID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactDraft {
    pub case_id: CaseId,
    pub actor: UserId,
    pub command: ProceduralFactCommand,
    pub result_revision: FactRevision,
    pub values: ProceduralFactValues,
    pub values_digest: Sha256Digest,
    pub sources: FactSources,
    pub sources_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
    /// Observation only, never an expected revision or a requirement for a full profile.
    pub observed_administration: CurrentCaseAdministration,
}
