use crate::{
    deadline_inputs::DeadlineInputMaterial,
    deadline_profiles::DeadlineProfileDetail,
    deadline_reevaluation::{TechnicalCause, TechnicalService},
    deadlines::{
        DeadlineActorSnapshot, DeadlineCalculation, DeadlineDefinition, DeadlineDetail,
        DeadlineOperationId, DeadlineReceipt, DeadlineRevision, DeadlineTrackingCapture,
    },
    procedural_facts::FactDetail,
};
use domain::{clock::OffsetDateTime, procedural_facts::FactText};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineReevaluationCommand {
    pub operation_id: DeadlineOperationId,
    pub cause: TechnicalCause,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineReevaluationInputs {
    pub profile_head: DeadlineProfileDetail,
    pub material: DeadlineInputMaterial,
    pub notification_parent_head: Option<FactDetail>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineReevaluationNoChange {
    Retired,
    AlreadyObserved,
    DependencyNotSelected,
    AlreadyInitialized,
}

#[derive(Debug)]
pub enum DeadlineReevaluationOutcome {
    NoChange(DeadlineReevaluationNoChange),
    Revision(Box<PreparedDeadlineReevaluation>),
}

/// Validated technical preparation retains the base and exact examined inputs.
#[derive(Debug)]
pub struct PreparedDeadlineReevaluation {
    pub(super) base: DeadlineDetail,
    pub(super) inputs: DeadlineReevaluationInputs,
    pub(super) definition: DeadlineDefinition,
    pub(super) calculation: DeadlineCalculation,
    pub(super) tracking: DeadlineTrackingCapture,
    pub(super) receipt: DeadlineReceipt,
    pub(super) revision: DeadlineRevision,
    pub(super) reason: FactText,
}

impl PreparedDeadlineReevaluation {
    pub fn base(&self) -> &DeadlineDetail {
        &self.base
    }

    pub fn inputs(&self) -> &DeadlineReevaluationInputs {
        &self.inputs
    }

    pub fn definition(&self) -> &DeadlineDefinition {
        &self.definition
    }

    pub fn calculation(&self) -> &DeadlineCalculation {
        &self.calculation
    }

    pub fn tracking(&self) -> &DeadlineTrackingCapture {
        &self.tracking
    }

    pub fn receipt(&self) -> &DeadlineReceipt {
        &self.receipt
    }

    /// The persistence caller supplies the recording time when committing.
    pub fn record(&self, recorded_at: OffsetDateTime) -> DeadlineDetail {
        DeadlineDetail {
            id: self.base.id,
            case_id: self.base.case_id,
            revision: self.revision,
            definition: self.definition.clone(),
            calculation: self.calculation.clone(),
            tracking: Some(self.tracking.clone()),
            responsible: self.base.responsible.clone(),
            attention: self.base.attention.clone(),
            status: self.base.status,
            reason: Some(self.reason.clone()),
            receipt: self.receipt.clone(),
            recorded_at,
            recorded_by: technical_author(),
        }
    }
}

pub(super) fn technical_author() -> DeadlineActorSnapshot {
    DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    }
}
