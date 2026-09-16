use super::{FactEvidence, FactLabel, FactText};
use crate::{
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
};

/// Selection only; a repository must resolve this exact result and agreement membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactHearingRef {
    pub hearing_id: HearingId,
    pub result_id: HearingResultId,
    pub revision: HearingResultRevision,
    pub agreement_id: Option<HearingResultAgreementId>,
}

/// A statement's source, independent of whether an admitted file is available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactProvenance {
    OperatorNote {
        note: FactText,
    },
    ExternalReference {
        reference: FactText,
        support: Option<FactEvidence>,
    },
    HearingResult {
        reference: FactHearingRef,
        locator: FactLabel,
        support: Option<FactEvidence>,
    },
}
impl FactProvenance {
    pub const fn support(&self) -> Option<&FactEvidence> {
        match self {
            Self::OperatorNote { .. } => None,
            Self::ExternalReference { support, .. } | Self::HearingResult { support, .. } => {
                support.as_ref()
            }
        }
    }
    pub const fn hearing_reference(&self) -> Option<FactHearingRef> {
        match self {
            Self::HearingResult { reference, .. } => Some(*reference),
            _ => None,
        }
    }
}
