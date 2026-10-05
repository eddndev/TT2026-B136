use super::MeasureAdministrativeRef;
use crate::{
    precautionary_hearings::{PrecautionaryHearingCapture, PrecautionaryHearingOrigin},
    precautionary_measures::{MeasureDecisionRecordHistoryEvidence, MeasureGroupRef},
};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    precautionary_hearings::{
        PrecautionaryHearingId, PrecautionaryHearingOperationId, PrecautionaryHearingRevision,
        PrecautionaryMeasureRef,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeHearingHistory {
    pub origin: PrecautionaryHearingOrigin,
    pub captures: Vec<PrecautionaryHearingCapture>,
}

/// Supplied owners and hearing prefixes; durable completeness is not implied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeDependencyInventory {
    pub records: MeasureDecisionRecordHistoryEvidence,
    pub hearings: Vec<MeasureAdministrativeHearingHistory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureDependencyJudicialOwner {
    V1(MeasureGroupRef),
    V2(MeasureGroupRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDependencyHearingRef {
    pub hearing_id: PrecautionaryHearingId,
    pub revision: PrecautionaryHearingRevision,
    pub operation_id: PrecautionaryHearingOperationId,
    pub capture_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureAdministrativeDependant {
    Judicial {
        owner: MeasureDependencyJudicialOwner,
        target: PrecautionaryMeasureRef,
    },
    Administrative {
        owner: MeasureAdministrativeRef,
        target: PrecautionaryMeasureRef,
    },
    Review {
        hearing: MeasureDependencyHearingRef,
        target: PrecautionaryMeasureRef,
    },
    ReviewAnchor {
        owner: MeasureDependencyJudicialOwner,
        hearing: MeasureDependencyHearingRef,
        target: PrecautionaryMeasureRef,
    },
}

/// Direct uses in validated supplied evidence, not current mutation permission.
#[derive(Debug)]
pub struct CheckedMeasureAdministrativeDependencies {
    pub(super) case_id: CaseId,
    pub(super) target: PrecautionaryMeasureRef,
    pub(super) dependants: Vec<MeasureAdministrativeDependant>,
}

impl CheckedMeasureAdministrativeDependencies {
    pub fn case_id(&self) -> CaseId {
        self.case_id
    }

    pub fn target(&self) -> PrecautionaryMeasureRef {
        self.target
    }

    pub fn dependants(&self) -> &[MeasureAdministrativeDependant] {
        &self.dependants
    }
}
