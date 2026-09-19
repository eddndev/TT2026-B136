use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    deadlines::{DeadlineId, DeadlineOperationId},
    identity::UserId,
    typed_participants::Uuid,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TrackedAction {
    Register = 0,
    Correct = 1,
    SetAttention = 2,
    Retire = 3,
    Reevaluate = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TechnicalService {
    DeadlineReevaluator = 0,
}

/// Technical authors are a service identity, never a fabricated user account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackedAuthor {
    User {
        id: UserId,
        email: String,
    },
    Technical {
        service: TechnicalService,
        policy_version: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredecessorReceipt {
    pub submission_digest: Sha256Digest,
    pub capture_digest: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DependencyFamily {
    Resolution = 0,
    Notification = 1,
    HearingResult = 2,
    Calendar = 3,
    Profile = 4,
}

/// Exact immutable source event; persistence also verifies its sequence and scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceEventReference {
    pub sequence: u64,
    pub family: DependencyFamily,
    pub source_id: Uuid,
    pub revision: u32,
    pub case_id: Option<CaseId>,
    pub hearing_id: Option<Uuid>,
    pub operation_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechnicalCause {
    SourceEvent {
        job_id: Uuid,
        event: SourceEventReference,
    },
    LegacyBootstrap {
        job_id: Uuid,
        policy_version: u16,
    },
}

/// The reviewed observation commitment is distinct from the historical calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedSubmission {
    pub case_id: CaseId,
    pub deadline_id: DeadlineId,
    pub operation_id: DeadlineOperationId,
    pub action: TrackedAction,
    pub expected_revision: u32,
    pub review_digest: Sha256Digest,
    pub observations_digest: Sha256Digest,
    pub predecessor: Option<PredecessorReceipt>,
    pub author: TrackedAuthor,
    pub reason: Option<String>,
    /// None encodes an explicitly manual cause, not an unknown technical cause.
    pub cause: Option<TechnicalCause>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ObservationRole {
    Profile = 0,
    Source = 1,
    Calendar = 2,
    NotificationParent = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionReference {
    pub id: Uuid,
    pub revision: u32,
}

/// Digests bind exact historical receipts and reconstructed readable evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationEntry {
    pub role: ObservationRole,
    pub family: DependencyFamily,
    pub id: Uuid,
    pub revision: u32,
    pub case_id: Option<CaseId>,
    pub hearing_id: Option<Uuid>,
    pub parent_resolution: Option<ResolutionReference>,
    pub submission_digest: Sha256Digest,
    pub evidence_digest: Sha256Digest,
}

/// Entries remain in strict role order; decoding never sorts or deduplicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observations {
    pub case_id: CaseId,
    pub entries: Vec<ObservationEntry>,
}

impl TrackedAuthor {
    pub const fn user_id(&self) -> Option<UserId> {
        match self {
            Self::User { id, .. } => Some(*id),
            Self::Technical { .. } => None,
        }
    }
    pub fn email(&self) -> Option<&str> {
        match self {
            Self::User { email, .. } => Some(email),
            Self::Technical { .. } => None,
        }
    }
}
