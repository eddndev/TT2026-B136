use domain::case_administration::CaseRevision;
use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentHasher, DocumentVersionRef, Sha256Digest};

use super::{
    CaseStage, CaseStageChange, CaseStageRevision, StageDocumentFormat, StageFormatPolicy,
};
use crate::cases::{CaseActorSnapshot, CaseInitialStageRegistration};
use crate::documents::DocumentRecord;

/// Immutable evidence of which exact file was admitted under the format policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSupportSnapshot {
    pub reference: DocumentVersionRef,
    pub digest: Sha256Digest,
    pub name: String,
    pub format: StageDocumentFormat,
    pub policy: StageFormatPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseStageSnapshot {
    pub case_id: CaseId,
    pub stage_revision: CaseStageRevision,
    pub from_stage: Option<CaseStage>,
    pub values: CaseStageChange,
    pub values_digest: Sha256Digest,
    pub administration_revision: CaseRevision,
    pub administration_digest: Sha256Digest,
    pub supports: Vec<StageSupportSnapshot>,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: CaseActorSnapshot,
}

/// Initial registration retains its original administrative digest and provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseStageEntry {
    Initial(CaseInitialStageRegistration),
    Changed(Box<CaseStageSnapshot>),
}

impl CaseStageEntry {
    pub fn case_id(&self) -> CaseId {
        match self {
            Self::Initial(value) => value.case_id,
            Self::Changed(value) => value.case_id,
        }
    }
    pub fn stage_revision(&self) -> CaseStageRevision {
        match self {
            Self::Initial(value) => value.stage_revision,
            Self::Changed(value) => value.stage_revision,
        }
    }
    pub fn stage(&self) -> CaseStage {
        match self {
            Self::Initial(_) => CaseStage::Investigation,
            Self::Changed(value) => value.values.stage(),
        }
    }
    pub fn recorded_at(&self) -> OffsetDateTime {
        match self {
            Self::Initial(value) => value.recorded_at,
            Self::Changed(value) => value.recorded_at,
        }
    }
    pub fn recorded_by(&self) -> &CaseActorSnapshot {
        match self {
            Self::Initial(value) => &value.recorded_by,
            Self::Changed(value) => &value.recorded_by,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrentCaseStage {
    Unregistered,
    Registered(Box<CaseStageEntry>),
}

impl CurrentCaseStage {
    pub fn entry(&self) -> Option<&CaseStageEntry> {
        match self {
            Self::Unregistered => None,
            Self::Registered(entry) => Some(entry),
        }
    }
    pub fn revision(&self) -> Option<CaseStageRevision> {
        self.entry().map(CaseStageEntry::stage_revision)
    }
    pub fn stage(&self) -> Option<CaseStage> {
        self.entry().map(CaseStageEntry::stage)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseStageDetail {
    pub case_id: CaseId,
    pub current: CurrentCaseStage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseStagePage {
    pub entries: Vec<CaseStageEntry>,
    pub has_more: bool,
    pub next_before_revision: Option<CaseStageRevision>,
}

/// Internal preparation result, with no public read event or plaintext response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseStagePreparation {
    pub current: CurrentCaseStage,
    pub records: Vec<DocumentRecord>,
}

pub fn case_stage_digest(hasher: &dyn DocumentHasher, values: &CaseStageChange) -> Sha256Digest {
    hasher.hash_bytes(&values.canonical_bytes())
}
