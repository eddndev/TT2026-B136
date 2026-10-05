use super::*;
use crate::{
    case_stages::StageSupportSnapshot, precautionary_hearings::PrecautionaryContext,
    precautionary_measures::*,
};
use domain::{
    clock::OffsetDateTime, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::MeasureValues,
};

#[derive(Clone, Copy)]
pub(super) enum JudicialView<'a> {
    V1(&'a MeasureDecisionGroupCapture, usize),
    V2(&'a MeasureDecisionGroupCaptureV2, usize),
}
impl<'a> JudicialView<'a> {
    pub fn reference(self) -> PrecautionaryMeasureRef {
        match self {
            Self::V1(g, i) => {
                let m = &g.measures[i];
                PrecautionaryMeasureRef::new(m.result.id, m.result.revision, m.capture_digest)
            }
            Self::V2(g, i) => {
                let m = &g.measures[i];
                PrecautionaryMeasureRef::new(m.result.id, m.result.revision, m.capture_digest)
            }
        }
    }
    pub fn context(self) -> &'a PrecautionaryContext {
        match self {
            Self::V1(g, _) => &g.review.material.context,
            Self::V2(g, _) => &g.review.material.context,
        }
    }
    pub fn support(self) -> &'a StageSupportSnapshot {
        match self {
            Self::V1(g, _) => &g.decision.support,
            Self::V2(g, _) => &g.decision.support,
        }
    }
    pub fn recorded_at(self) -> OffsetDateTime {
        match self {
            Self::V1(g, i) => g.measures[i].recorded_at,
            Self::V2(g, i) => g.measures[i].recorded_at,
        }
    }
    pub fn values(self) -> &'a MeasureValues {
        match self {
            Self::V1(g, i) => &g.measures[i].result.values,
            Self::V2(g, i) => &g.measures[i].result.values,
        }
    }
    pub fn sources(self) -> &'a MeasureSources {
        match self {
            Self::V1(g, i) => &g.measures[i].result.sources,
            Self::V2(g, i) => &g.measures[i].result.sources,
        }
    }
    pub fn projection(self) -> &'a MeasureSourceProjection {
        match self {
            Self::V1(g, i) => &g.measures[i].result.projection,
            Self::V2(g, i) => &g.measures[i].result.projection,
        }
    }
    pub fn judicial_origin(self) -> MeasureOriginIds {
        match self {
            Self::V1(g, i) => g.measures[i].result.origin,
            Self::V2(g, i) => g.measures[i].result.judicial_origin,
        }
    }
    pub fn root(self) -> MeasureRecordRoot {
        match self {
            Self::V1(g, i) => MeasureRecordRoot::Judicial(g.measures[i].result.origin),
            Self::V2(g, i) => g.measures[i].result.record_root.clone(),
        }
    }
    pub fn action(self) -> MeasureCaptureAction {
        match self {
            Self::V1(g, i) => g.measures[i].result.action,
            Self::V2(g, i) => g.measures[i].result.action,
        }
    }
    pub fn owner(self) -> MeasureGroupRef {
        match self {
            Self::V1(g, _) => MeasureGroupRef {
                operation_id: g.review.command.operation_id,
                decision_id: g.review.command.decision_id,
                group_digest: g.capture_digest,
            },
            Self::V2(g, _) => MeasureGroupRef {
                operation_id: g.review.command.operation_id,
                decision_id: g.review.command.decision_id,
                group_digest: g.capture_digest,
            },
        }
    }
    pub fn to_owned(self) -> OwnedJudicialMeasure {
        match self {
            Self::V1(g, i) => OwnedJudicialMeasure::V1(Box::new(OwnedMeasureMaterial {
                owner: self.owner(),
                capture: g.measures[i].clone(),
            })),
            Self::V2(g, i) => OwnedJudicialMeasure::V2(Box::new(OwnedMeasureMaterialV2 {
                owner: self.owner(),
                capture: g.measures[i].clone(),
            })),
        }
    }
}
