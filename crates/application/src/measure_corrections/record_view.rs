use super::*;
use crate::{
    case_stages::StageSupportSnapshot, precautionary_hearings::PrecautionaryContext,
    precautionary_measures::*,
};
use domain::{
    clock::OffsetDateTime, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::MeasureValues,
};

/// Borrowed material is exposed internally only after parent-first reconstruction.
#[derive(Clone, Copy)]
pub(super) struct RecordView<'a> {
    pub group: &'a MeasureDecisionGroupCapture,
    pub member: usize,
    pub administrative: Option<&'a MeasureAdministrativeCapture>,
}
impl<'a> RecordView<'a> {
    pub fn judicial(&self) -> &'a MeasureCapture {
        &self.group.measures[self.member]
    }
    fn row(&self) -> Option<&'a MeasureAdministrativeRecordCapture> {
        self.administrative.map(|a| &a.records[0])
    }
    pub fn reference(&self) -> PrecautionaryMeasureRef {
        if let Some(c) = self.row() {
            PrecautionaryMeasureRef::new(c.result.id, c.result.revision, c.capture_digest)
        } else {
            let m = self.judicial();
            PrecautionaryMeasureRef::new(m.result.id, m.result.revision, m.capture_digest)
        }
    }
    pub fn context(&self) -> &'a PrecautionaryContext {
        self.row()
            .map_or(&self.group.review.material.context, |c| &c.context)
    }
    pub fn support(&self) -> &'a StageSupportSnapshot {
        &self.group.decision.support
    }
    pub fn recorded_at(&self) -> OffsetDateTime {
        self.row()
            .map_or(self.judicial().recorded_at, |c| c.recorded_at)
    }
    pub fn values(&self) -> &'a MeasureValues {
        self.row()
            .map_or(&self.judicial().result.values, |c| &c.result.values)
    }
    pub fn sources(&self) -> &'a MeasureSources {
        self.row()
            .map_or(&self.judicial().result.sources, |c| &c.result.sources)
    }
    pub fn record_root(&self) -> MeasureRecordRoot {
        self.row().map_or_else(
            || MeasureRecordRoot::Judicial(self.judicial().result.origin),
            |c| c.result.record_root.clone(),
        )
    }
    pub fn validity(&self) -> MeasureCaptureValidity {
        self.row()
            .map_or(MeasureCaptureValidity::Valid, |c| c.result.validity)
    }
    pub fn judicial_reference(&self) -> MeasureJudicialRef {
        let m = self.judicial();
        MeasureJudicialRef {
            owner: MeasureGroupRef {
                operation_id: self.group.review.command.operation_id,
                decision_id: self.group.review.command.decision_id,
                group_digest: self.group.capture_digest,
            },
            reference: PrecautionaryMeasureRef::new(
                m.result.id,
                m.result.revision,
                m.capture_digest,
            ),
        }
    }
    pub fn to_owned(self) -> ResolvedMeasureRecord {
        let last_judicial = OwnedMeasureMaterial {
            owner: self.judicial_reference().owner,
            capture: self.judicial().clone(),
        };
        let record = if let Some(a) = self.administrative {
            OwnedMeasureRecord::Administrative {
                owner: MeasureAdministrativeRef {
                    operation_id: a.review.command.operation_id,
                    capture_digest: a.capture_digest,
                },
                capture: Box::new(a.records[0].clone()),
            }
        } else {
            OwnedMeasureRecord::Judicial(Box::new(last_judicial.clone()))
        };
        ResolvedMeasureRecord {
            record,
            context: self.context().clone(),
            support: self.support().clone(),
            last_judicial,
        }
    }
}
