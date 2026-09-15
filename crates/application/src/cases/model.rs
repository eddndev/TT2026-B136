use domain::cases::{CaseId, CaseMetadata};
use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::UserId;

use super::{
    CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision, CaseStageRevision,
    InitialCaseStage,
};

/// Identity captured at commit, independent of subsequent account edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseActorSnapshot {
    pub id: UserId,
    pub email: String,
}

/// Original case identity and creation facts, unchanged by administration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseOrigin {
    pub id: CaseId,
    pub created_by: UserId,
    pub created_at: OffsetDateTime,
}

/// One positive immutable revision with its original author and time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseAdministrationSnapshot {
    pub case_id: CaseId,
    pub revision: CaseRevision,
    pub values: CaseAdministrationValues,
    pub values_digest: Sha256Digest,
    pub changed_at: OffsetDateTime,
    pub changed_by: CaseActorSnapshot,
}

/// A baseline has no fabricated revision, digest, author or change time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrentCaseAdministration {
    Unrevised(CaseMetadata),
    Recorded(Box<CaseAdministrationSnapshot>),
}
impl CurrentCaseAdministration {
    pub fn revision(&self) -> Option<CaseRevision> {
        self.snapshot().map(|s| s.revision)
    }
    pub fn values(&self) -> CaseAdministrationValues {
        match self {
            Self::Unrevised(metadata) => CaseAdministrationValues::basic(metadata.clone()),
            Self::Recorded(snapshot) => snapshot.values.clone(),
        }
    }
    pub fn snapshot(&self) -> Option<&CaseAdministrationSnapshot> {
        match self {
            Self::Unrevised(_) => None,
            Self::Recorded(snapshot) => Some(snapshot),
        }
    }
}

/// Compact identifiers for staff, excluding the other sensitive profile fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasePenalIdentifiers {
    pub nuc: String,
    pub judicial_case_number: String,
}

/// A staff index projection; the basic client projection remains CaseRecord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseAdministrationOverview {
    pub origin: CaseOrigin,
    pub metadata: CaseMetadata,
    pub revision: Option<CaseRevision>,
    pub administrative_status: CaseAdministrativeStatus,
    pub penal_identifiers: Option<CasePenalIdentifiers>,
    pub initial_stage: Option<InitialCaseStage>,
}

/// Initial investigation registration bound to exact administrative revision one.
///
/// Persistence requires both counters to equal FIRST. The actor, time and digest
/// come from that immutable revision, never the current head or account profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseInitialStageRegistration {
    pub case_id: CaseId,
    pub stage_revision: CaseStageRevision,
    pub administration_revision: CaseRevision,
    pub stage: InitialCaseStage,
    pub administration_digest: Sha256Digest,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: CaseActorSnapshot,
}

/// Detailed staff view captured with administration and the initial stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseAdministrationDetail {
    pub origin: CaseOrigin,
    pub administration: CurrentCaseAdministration,
    pub initial_stage: Option<CaseInitialStageRegistration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseAdministrationPage {
    pub cases: Vec<CaseAdministrationOverview>,
    pub has_more: bool,
    pub next_after_id: Option<CaseId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseAdministrationHistoryPage {
    pub revisions: Vec<CaseAdministrationSnapshot>,
    pub has_more: bool,
    pub next_before_revision: Option<CaseRevision>,
}

/// Hashes bounded values only through the existing SHA-256 port.
pub fn case_administration_digest(
    hasher: &dyn DocumentHasher,
    values: &CaseAdministrationValues,
) -> Sha256Digest {
    hasher.hash_bytes(&values.canonical_bytes())
}
