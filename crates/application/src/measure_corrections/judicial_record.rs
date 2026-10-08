use super::MeasureRecordRoot;
use crate::{identity::Principal, precautionary_measures::*};
use domain::{
    cases::CaseId, clock::OffsetDateTime, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::MeasureValues,
};

/// The actual judicial capture family is retained without substituting corrected terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedJudicialMeasure {
    V1(Box<OwnedMeasureMaterial>),
    V2(Box<OwnedMeasureMaterialV2>),
}
impl OwnedJudicialMeasure {
    pub fn owner(&self) -> &MeasureGroupRef {
        match self {
            Self::V1(m) => &m.owner,
            Self::V2(m) => &m.owner,
        }
    }
    pub fn reference(&self) -> PrecautionaryMeasureRef {
        match self {
            Self::V1(m) => PrecautionaryMeasureRef::new(
                m.capture.result.id,
                m.capture.result.revision,
                m.capture.capture_digest,
            ),
            Self::V2(m) => PrecautionaryMeasureRef::new(
                m.capture.result.id,
                m.capture.result.revision,
                m.capture.capture_digest,
            ),
        }
    }
    pub fn case_id(&self) -> CaseId {
        match self {
            Self::V1(m) => m.capture.case_id,
            Self::V2(m) => m.capture.case_id,
        }
    }
    pub fn values(&self) -> &MeasureValues {
        match self {
            Self::V1(m) => &m.capture.result.values,
            Self::V2(m) => &m.capture.result.values,
        }
    }
    pub fn sources(&self) -> &MeasureSources {
        match self {
            Self::V1(m) => &m.capture.result.sources,
            Self::V2(m) => &m.capture.result.sources,
        }
    }
    pub fn projection(&self) -> &MeasureSourceProjection {
        match self {
            Self::V1(m) => &m.capture.result.projection,
            Self::V2(m) => &m.capture.result.projection,
        }
    }
    pub fn record_root(&self) -> MeasureRecordRoot {
        match self {
            Self::V1(m) => MeasureRecordRoot::Judicial(m.capture.result.origin),
            Self::V2(m) => m.capture.result.record_root.clone(),
        }
    }
    pub fn judicial_origin(&self) -> MeasureOriginIds {
        match self {
            Self::V1(m) => m.capture.result.origin,
            Self::V2(m) => m.capture.result.judicial_origin,
        }
    }
    pub fn last_action(&self) -> MeasureCaptureAction {
        match self {
            Self::V1(m) => m.capture.result.action,
            Self::V2(m) => m.capture.result.action,
        }
    }
    pub fn actor(&self) -> &Principal {
        match self {
            Self::V1(m) => &m.capture.actor,
            Self::V2(m) => &m.capture.actor,
        }
    }
    pub fn recorded_at(&self) -> OffsetDateTime {
        match self {
            Self::V1(m) => m.capture.recorded_at,
            Self::V2(m) => m.capture.recorded_at,
        }
    }
}
