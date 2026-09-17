use super::*;
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    identity::UserId,
    procedural_facts::{FactLabel, FactText},
};

/// An explicit authorization context; a case collection also includes global profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineProfileCollection {
    Global,
    ForCase(CaseId),
}
impl DeadlineProfileCollection {
    pub fn includes(self, scope: &DeadlineProfileScope) -> bool {
        match (self, scope) {
            (_, DeadlineProfileScope::Global(_)) => true,
            (Self::ForCase(case), DeadlineProfileScope::Case(id)) => case == *id,
            _ => false,
        }
    }
    /// Global mutations require the global context, including when read from a case.
    pub fn permits_mutation(self, scope: &DeadlineProfileScope) -> bool {
        match (self, scope) {
            (Self::Global, DeadlineProfileScope::Global(_)) => true,
            (Self::ForCase(case), DeadlineProfileScope::Case(id)) => case == *id,
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineProfileAction {
    Publish,
    Replace,
    Retire,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineProfileChange {
    Publish {
        definition: DeadlineProfileDefinition,
    },
    Replace {
        expected_revision: DeadlineProfileRevision,
        definition: DeadlineProfileDefinition,
        reason: FactText,
    },
    Retire {
        expected_revision: DeadlineProfileRevision,
        reason: FactText,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileCommand {
    pub operation_id: DeadlineProfileOperationId,
    pub profile_id: DeadlineProfileId,
    pub change: DeadlineProfileChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileActorSnapshot {
    pub id: UserId,
    pub email: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileReceipt {
    pub operation_id: DeadlineProfileOperationId,
    pub action: DeadlineProfileAction,
    pub expected_revision: u32,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileDetail {
    pub id: DeadlineProfileId,
    pub revision: DeadlineProfileRevision,
    pub definition: DeadlineProfileDefinition,
    pub definition_digest: Sha256Digest,
    pub algorithm: DeadlineProfileAlgorithm,
    pub status: DeadlineProfileStatus,
    pub reason: Option<FactText>,
    pub receipt: DeadlineProfileReceipt,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: DeadlineProfileActorSnapshot,
}
/// Listing never expands the example corpus or embedded example calendars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileOverview {
    pub id: DeadlineProfileId,
    pub revision: DeadlineProfileRevision,
    pub status: DeadlineProfileStatus,
    pub algorithm: DeadlineProfileAlgorithm,
    pub definition_digest: Sha256Digest,
    pub title: FactLabel,
    pub scope: DeadlineProfileScope,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfilePage {
    pub profiles: Vec<DeadlineProfileOverview>,
    pub has_more: bool,
    pub next_after_id: Option<DeadlineProfileId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileHistoryEntry {
    pub id: DeadlineProfileId,
    pub revision: DeadlineProfileRevision,
    pub status: DeadlineProfileStatus,
    pub algorithm: DeadlineProfileAlgorithm,
    pub definition_digest: Sha256Digest,
    pub scope: DeadlineProfileScope,
    pub reason: Option<FactText>,
    pub receipt: DeadlineProfileReceipt,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: DeadlineProfileActorSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileHistoryPage {
    pub revisions: Vec<DeadlineProfileHistoryEntry>,
    pub has_more: bool,
    pub next_before_revision: Option<DeadlineProfileRevision>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineProfileDraft {
    pub collection: DeadlineProfileCollection,
    pub actor: UserId,
    pub command: DeadlineProfileCommand,
    pub result_revision: DeadlineProfileRevision,
    pub initial_scope: DeadlineProfileScope,
    pub definition: DeadlineProfileDefinition,
    pub definition_digest: Sha256Digest,
    pub algorithm: DeadlineProfileAlgorithm,
    pub submission_digest: Sha256Digest,
}
