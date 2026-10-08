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
pub(crate) struct RecordView<'a> {
    pub(super) judicial: super::judicial_view::JudicialView<'a>,
    pub(super) administrative: Option<(&'a MeasureAdministrativeCapture, usize)>,
}
impl<'a> RecordView<'a> {
    fn row(&self) -> Option<&'a MeasureAdministrativeRecordCapture> {
        self.administrative.map(|(a, row)| &a.records[row])
    }
    pub fn reference(&self) -> PrecautionaryMeasureRef {
        if let Some(c) = self.row() {
            PrecautionaryMeasureRef::new(c.result.id, c.result.revision, c.capture_digest)
        } else {
            self.judicial.reference()
        }
    }
    pub fn context(&self) -> &'a PrecautionaryContext {
        self.row().map_or(self.judicial.context(), |c| &c.context)
    }
    pub fn support(&self) -> &'a StageSupportSnapshot {
        self.judicial.support()
    }
    pub fn recorded_at(&self) -> OffsetDateTime {
        self.row()
            .map_or(self.judicial.recorded_at(), |c| c.recorded_at)
    }
    pub fn values(&self) -> &'a MeasureValues {
        self.row()
            .map_or(self.judicial.values(), |c| &c.result.values)
    }
    pub fn sources(&self) -> &'a MeasureSources {
        self.row()
            .map_or(self.judicial.sources(), |c| &c.result.sources)
    }
    pub fn record_root(&self) -> MeasureRecordRoot {
        self.row()
            .map_or_else(|| self.judicial.root(), |c| c.result.record_root.clone())
    }
    pub fn validity(&self) -> MeasureCaptureValidity {
        self.row()
            .map_or(MeasureCaptureValidity::Valid, |c| c.result.validity)
    }
    pub fn judicial_reference(&self) -> MeasureJudicialRef {
        MeasureJudicialRef {
            owner: self.judicial.owner(),
            reference: self.judicial.reference(),
        }
    }
    pub fn judicial_origin(&self) -> MeasureOriginIds {
        self.judicial.judicial_origin()
    }
    pub fn last_action(&self) -> MeasureCaptureAction {
        self.judicial.action()
    }
    pub fn projection(&self) -> &'a MeasureSourceProjection {
        self.row()
            .map_or(self.judicial.projection(), |c| &c.result.projection)
    }
    pub fn to_owned(self) -> ResolvedMeasureRecord {
        let last_judicial = self.judicial.to_owned();
        let record = if let Some((a, row)) = self.administrative {
            OwnedMeasureRecord::Administrative {
                owner: MeasureAdministrativeRef {
                    operation_id: a.review.command.operation_id,
                    capture_digest: a.capture_digest,
                },
                capture: Box::new(a.records[row].clone()),
            }
        } else {
            OwnedMeasureRecord::Judicial(last_judicial.clone())
        };
        ResolvedMeasureRecord {
            record,
            context: self.context().clone(),
            support: self.support().clone(),
            last_judicial,
        }
    }
}
