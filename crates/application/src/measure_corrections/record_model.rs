use super::*;
use crate::{
    case_stages::StageSupportSnapshot, precautionary_hearings::PrecautionaryContext,
    precautionary_measures::*,
};
use domain::{
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::{MeasureCorrectionOperationId, MeasureValues},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeRef {
    pub operation_id: MeasureCorrectionOperationId,
    pub capture_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedMeasureRecord {
    Judicial(Box<OwnedMeasureMaterial>),
    Administrative {
        owner: MeasureAdministrativeRef,
        capture: Box<MeasureAdministrativeRecordCapture>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeEvidence {
    pub origin: MeasureAdministrativeOrigin,
    pub capture: MeasureAdministrativeCapture,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureRecordHistoryEvidence {
    pub judicial: MeasureHistoryEvidence,
    pub administrative: Vec<MeasureAdministrativeEvidence>,
}

/// Exact supplied history has been reconstructed; live heads remain a store obligation.
#[derive(Debug)]
pub struct ResolvedMeasureRecord {
    pub(super) record: OwnedMeasureRecord,
    pub(super) context: PrecautionaryContext,
    pub(super) support: StageSupportSnapshot,
    pub(super) last_judicial: OwnedMeasureMaterial,
}
impl ResolvedMeasureRecord {
    pub fn record(&self) -> &OwnedMeasureRecord {
        &self.record
    }
    pub fn context(&self) -> &PrecautionaryContext {
        &self.context
    }
    pub fn support(&self) -> &StageSupportSnapshot {
        &self.support
    }
    pub fn last_judicial(&self) -> &OwnedMeasureMaterial {
        &self.last_judicial
    }
    pub fn reference(&self) -> PrecautionaryMeasureRef {
        match &self.record {
            OwnedMeasureRecord::Judicial(m) => PrecautionaryMeasureRef::new(
                m.capture.result.id,
                m.capture.result.revision,
                m.capture.capture_digest,
            ),
            OwnedMeasureRecord::Administrative { capture: c, .. } => {
                PrecautionaryMeasureRef::new(c.result.id, c.result.revision, c.capture_digest)
            }
        }
    }
    pub fn recorded_at(&self) -> OffsetDateTime {
        match &self.record {
            OwnedMeasureRecord::Judicial(m) => m.capture.recorded_at,
            OwnedMeasureRecord::Administrative { capture, .. } => capture.recorded_at,
        }
    }
    pub fn values(&self) -> &MeasureValues {
        match &self.record {
            OwnedMeasureRecord::Judicial(m) => &m.capture.result.values,
            OwnedMeasureRecord::Administrative { capture, .. } => &capture.result.values,
        }
    }
    pub fn sources(&self) -> &MeasureSources {
        match &self.record {
            OwnedMeasureRecord::Judicial(m) => &m.capture.result.sources,
            OwnedMeasureRecord::Administrative { capture, .. } => &capture.result.sources,
        }
    }
    pub fn projection(&self) -> &MeasureSourceProjection {
        match &self.record {
            OwnedMeasureRecord::Judicial(m) => &m.capture.result.projection,
            OwnedMeasureRecord::Administrative { capture, .. } => &capture.result.projection,
        }
    }
    pub fn record_root(&self) -> MeasureRecordRoot {
        match &self.record {
            OwnedMeasureRecord::Judicial(m) => MeasureRecordRoot::Judicial(m.capture.result.origin),
            OwnedMeasureRecord::Administrative { capture, .. } => {
                capture.result.record_root.clone()
            }
        }
    }
    pub fn judicial_origin(&self) -> MeasureOriginIds {
        match &self.record {
            OwnedMeasureRecord::Judicial(m) => m.capture.result.origin,
            OwnedMeasureRecord::Administrative { capture, .. } => capture.result.judicial_origin,
        }
    }
    pub fn validity(&self) -> MeasureCaptureValidity {
        match &self.record {
            OwnedMeasureRecord::Judicial(_) => MeasureCaptureValidity::Valid,
            OwnedMeasureRecord::Administrative { capture, .. } => capture.result.validity,
        }
    }
    pub fn last_action(&self) -> MeasureCaptureAction {
        match &self.record {
            OwnedMeasureRecord::Judicial(m) => m.capture.result.action,
            OwnedMeasureRecord::Administrative { capture, .. } => capture.result.last_action,
        }
    }
}

#[derive(Debug)]
pub struct CheckedMeasureRecords {
    pub(super) targets: Vec<ResolvedMeasureRecord>,
}
impl CheckedMeasureRecords {
    pub fn targets(&self) -> &[ResolvedMeasureRecord] {
        &self.targets
    }
}
