use super::*;
use crate::ApplicationError;
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    identity::UserId,
    procedural_facts::{FactLabel, FactText},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineStatusFilter {
    All,
    #[default]
    Active,
    Retired,
}
impl DeadlineStatusFilter {
    pub const fn status(self) -> Option<DeadlineStatus> {
        match self {
            Self::All => None,
            Self::Active => Some(DeadlineStatus::Active),
            Self::Retired => Some(DeadlineStatus::Retired),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineQuery {
    limit: u32,
    after_id: Option<DeadlineId>,
    status: DeadlineStatusFilter,
}
impl DeadlineQuery {
    pub fn new(
        limit: u32,
        after_id: Option<DeadlineId>,
        status: DeadlineStatusFilter,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit, 100)?;
        Ok(Self {
            limit,
            after_id,
            status,
        })
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn after_id(&self) -> Option<DeadlineId> {
        self.after_id
    }
    pub const fn status(&self) -> DeadlineStatusFilter {
        self.status
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineHistoryQuery {
    limit: u32,
    before_revision: Option<DeadlineRevision>,
}
impl DeadlineHistoryQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        validate_limit(limit, 20)?;
        Ok(Self {
            limit,
            before_revision: before_revision.map(DeadlineRevision::new).transpose()?,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn before_revision(self) -> Option<DeadlineRevision> {
        self.before_revision
    }
}
fn validate_limit(limit: u32, maximum: u32) -> Result<(), ApplicationError> {
    if !(1..=maximum).contains(&limit) {
        return Err(DeadlineError::Invalid("page limit").into());
    }
    Ok(())
}

/// Compact current state. The store validates it against the captured full record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineOverview {
    pub id: DeadlineId,
    pub case_id: CaseId,
    pub revision: DeadlineRevision,
    pub title: FactLabel,
    pub status: DeadlineStatus,
    pub responsible: DeadlineResponsibleSnapshot,
    pub attention_recorded: bool,
    pub due_at: Option<OffsetDateTime>,
    pub blocked: bool,
}
impl From<&DeadlineDetail> for DeadlineOverview {
    fn from(value: &DeadlineDetail) -> Self {
        Self {
            id: value.id,
            case_id: value.case_id,
            revision: value.revision,
            title: value.definition.title.clone(),
            status: value.status,
            responsible: value.responsible.clone(),
            attention_recorded: matches!(value.attention, DeadlineAttention::Recorded { .. }),
            due_at: value.calculation.result.due_at(),
            blocked: !value.calculation.result.blocks().is_empty(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlinePage {
    pub deadlines: Vec<DeadlineOverview>,
    pub has_more: bool,
    pub next_after_id: Option<DeadlineId>,
}

/// Lightweight operation receipt; an exact detail supplies its immutable calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineHistoryEntry {
    pub id: DeadlineId,
    pub case_id: CaseId,
    pub revision: DeadlineRevision,
    pub status: DeadlineStatus,
    pub reason: Option<FactText>,
    pub receipt: DeadlineReceipt,
    pub state_digest: Sha256Digest,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: DeadlineActorSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineHistoryPage {
    pub revisions: Vec<DeadlineHistoryEntry>,
    pub has_more: bool,
    pub next_before_revision: Option<DeadlineRevision>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineDraft {
    pub case_id: CaseId,
    pub actor: UserId,
    pub command: DeadlineCommand,
    pub result_revision: DeadlineRevision,
    pub definition: DeadlineDefinition,
    pub calculation: DeadlineCalculation,
    pub responsible: DeadlineResponsibleSnapshot,
    pub attention: DeadlineAttention,
    pub status: DeadlineStatus,
    pub review_digest: Sha256Digest,
    pub capture_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
}
