use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::UserId;

use super::{ParticipantId, ParticipantRevision, ParticipantValues};

/// Identity captured with a revision, independent of later profile changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantActorSnapshot {
    pub id: UserId,
    pub email: String,
}

/// One complete immutable revision and its original provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantSnapshot {
    pub case_id: CaseId,
    pub id: ParticipantId,
    pub revision: ParticipantRevision,
    pub values: ParticipantValues,
    pub values_digest: Sha256Digest,
    pub changed_at: OffsetDateTime,
    pub changed_by: ParticipantActorSnapshot,
}

/// Current snapshots with a stable, exclusive UUID pagination boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantPage {
    pub participants: Vec<ParticipantSnapshot>,
    pub has_more: bool,
    pub next_after_id: Option<ParticipantId>,
}

/// Immutable snapshots in descending revision order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantHistoryPage {
    pub revisions: Vec<ParticipantSnapshot>,
    pub has_more: bool,
    pub next_before_revision: Option<ParticipantRevision>,
}

/// Hashes values only through the SHA-256 port; provenance is separate.
pub fn participant_digest(hasher: &dyn DocumentHasher, values: &ParticipantValues) -> Sha256Digest {
    hasher.hash_bytes(&values.canonical_bytes())
}
