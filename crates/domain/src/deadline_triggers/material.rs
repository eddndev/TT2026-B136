use super::TriggerSourceRef;
use crate::{
    cases::CaseId,
    crypto::Sha256Digest,
    hearing_results::{
        HearingResultAgreement, HearingResultId, HearingResultProvenance, HearingResultRevision,
        HearingResultValues,
    },
    hearings::HearingId,
    procedural_facts::{
        FactProvenance, FactRevision, FactStatedEffect, NotificationRoot, NotificationValues,
        ResolutionRoot, ResolutionValues,
    },
};

/// Captured digests; the application must verify their contents before extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactTriggerDigests {
    pub values: Sha256Digest,
    pub sources: Sha256Digest,
    pub submission: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingTriggerDigests {
    pub values: Sha256Digest,
    pub submission: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerDigests {
    ProceduralFact(FactTriggerDigests),
    HearingResult(HearingTriggerDigests),
}

/// Borrowed exact values, not a proof of authorization, persistence or applicability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMaterial<'a> {
    Resolution {
        root: ResolutionRoot,
        revision: FactRevision,
        values: &'a ResolutionValues,
        digests: FactTriggerDigests,
    },
    Notification {
        root: NotificationRoot,
        revision: FactRevision,
        values: &'a NotificationValues,
        digests: FactTriggerDigests,
    },
    HearingResult {
        case_id: CaseId,
        hearing_id: HearingId,
        result_id: HearingResultId,
        revision: HearingResultRevision,
        values: &'a HearingResultValues,
        digests: HearingTriggerDigests,
    },
}
impl TriggerMaterial<'_> {
    pub const fn case_id(self) -> CaseId {
        match self {
            Self::Resolution { root, .. } => root.case_id(),
            Self::Notification { root, .. } => root.case_id(),
            Self::HearingResult { case_id, .. } => case_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerProvenance {
    ProceduralFact(FactProvenance),
    HearingResult(HearingResultProvenance),
}

/// Owns the exact source evidence retained by extraction, independently of later edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerSourceSnapshot {
    pub case_id: CaseId,
    pub reference: TriggerSourceRef,
    pub digests: TriggerDigests,
    pub provenance: TriggerProvenance,
    pub agreement: Option<HearingResultAgreement>,
    pub stated_effect: Option<FactStatedEffect>,
}
