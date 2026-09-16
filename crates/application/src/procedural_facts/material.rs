use super::{FactHearingRef, FactResolutionRef, ResolutionSnapshot};
use crate::{
    case_stages::StageSupportSnapshot,
    hearing_results::{HearingResultAgreement, HearingResultSnapshot, HearingResultText},
    typed_participants::{ParticipantDetail, ParticipantOverview},
};
use domain::{
    hearing_results::{DeclaredHearingResultTime, HearingResultOccurrence},
    procedural_facts::{FactDeclaration, FactLabel, FactText, ResolutionClass},
    procedural_time::DeclaredProceduralTime,
};

/// Bounded parent material without another resolution, directory or document batch.
/// Its captured support is historical admission, not a file to admit again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionSourceMaterial {
    pub snapshot: ResolutionSnapshot,
    pub hearing: Option<HearingResultSnapshot>,
    pub admitted_support: Option<StageSupportSnapshot>,
}

/// Exact server-resolved values for verification before deriving public projections.
/// The store must bound participants to four and result revisions to two. Results
/// deduplicate by hearing, result and revision; agreement selections stay in values.
/// Sources never recursively expand into DocumentRecord or another material graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactSourceMaterial {
    pub resolution: Option<Box<ResolutionSourceMaterial>>,
    pub participants: Vec<ParticipantDetail>,
    pub hearing_results: Vec<HearingResultSnapshot>,
}

/// Human-readable fields derived from the exact verified resolution values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactResolutionView {
    pub reference: FactResolutionRef,
    pub class: FactDeclaration<ResolutionClass>,
    pub issuer: FactDeclaration<FactLabel>,
    pub issued_at: DeclaredProceduralTime,
    pub summary: FactText,
}

/// The optional agreement belongs to this exact result revision, never its head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactHearingView {
    pub reference: FactHearingRef,
    pub occurrence: HearingResultOccurrence,
    pub event_time: DeclaredHearingResultTime,
    pub summary: HearingResultText,
    pub agreement: Option<HearingResultAgreement>,
}

/// Bounded readable projections; complete subject identities remain internal.
/// Derive these from verified material and compare them on historical reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactSourceViews {
    pub resolution: Option<FactResolutionView>,
    pub participants: Vec<ParticipantOverview>,
    pub hearing_results: Vec<FactHearingView>,
}
