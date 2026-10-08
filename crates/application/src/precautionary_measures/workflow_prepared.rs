use super::{workflow_evidence as evidence, *};
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    identity::Principal,
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher};
use std::sync::Arc;

/// Only the authorized service constructs this after exact encrypted support admission.
pub struct PreparedMeasureDecision {
    pub(super) checked: CheckedMeasureDecisionReview,
    pub(super) material: MeasureDecisionReady,
    actor: Principal,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
impl PreparedMeasureDecision {
    pub fn review(&self) -> &MeasureDecisionReview {
        self.checked.review()
    }
    pub fn material(&self) -> &MeasureDecisionReady {
        &self.material
    }
    pub fn actor(&self) -> &Principal {
        &self.actor
    }
    pub fn into_operation(
        self,
        at: OffsetDateTime,
    ) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
        let group = self.checked.into_group_capture(self.hasher.as_ref(), at)?;
        let measure_history = self.material.measure_history;
        let origin = measure_group_origin(self.hasher.as_ref(), &group, &measure_history)?;
        Ok(MeasureDecisionStoredOperation {
            group,
            origin,
            measure_history,
        })
    }
}

pub(super) struct PreparationServices<'a> {
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub processor: &'a DocumentProcessor,
    pub validator: &'a dyn DocumentFormatBatchValidator,
    pub limits: &'a StageSupportReadLimits,
}

pub(super) fn prepare(
    services: PreparationServices<'_>,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureDecisionCommand,
    material: MeasureDecisionReady,
) -> Result<PreparedMeasureDecision, ApplicationError> {
    super::decision_wire::bounded(material.predecessors.len())?;
    super::decision_wire::bounded(material.result_sources.len())?;
    super::anchor_validation::shape(&material.anchor)?;
    super::history_inventory::limits(
        &material.measure_history,
        1,
        command.outcome.affected_ids().len(),
    )?;
    if material.context.material().case_id != case_id {
        return Err(evidence::invalid("ready context belongs to another case"));
    }
    let support = admit_measure_decision_support(
        &command.values,
        std::slice::from_ref(&material.support_record),
        services.processor,
        services.limits,
        services.validator,
    )?;
    let selected = MeasureDecisionMaterial {
        context: material.context.clone(),
        support,
        anchor: material.anchor.clone(),
        predecessors: material.predecessors.clone(),
        result_sources: material.result_sources.clone(),
    };
    let checked = prepare_measure_decision_with_history(
        services.hasher.as_ref(),
        actor,
        case_id,
        command,
        selected,
        &material.measure_history,
    )?;
    Ok(PreparedMeasureDecision {
        checked,
        material,
        actor: actor.clone(),
        hasher: services.hasher,
    })
}
