use super::{record_workflow_evidence as evidence, workflow_evidence::invalid, *};
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    identity::Principal,
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher};
use std::sync::Arc;

/// Constructed only after authorization, exact support admission and mixed proof validation.
pub struct PreparedMeasureDecisionRecord {
    pub(super) checked: CheckedMeasureDecisionReviewV2,
    pub(super) material: MeasureDecisionRecordReady,
    actor: Principal,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}

impl PreparedMeasureDecisionRecord {
    pub fn actor(&self) -> &Principal {
        &self.actor
    }
    pub fn review(&self) -> &MeasureDecisionReviewV2 {
        self.checked.review()
    }
    pub fn material(&self) -> &MeasureDecisionRecordReady {
        &self.material
    }
    pub fn into_operation(
        self,
        at: OffsetDateTime,
    ) -> Result<MeasureDecisionRecordStoredOperation, ApplicationError> {
        let group = self.checked.into_group_capture(self.hasher.as_ref(), at)?;
        let record_history = self.material.record_history;
        let origin = measure_group_origin_v2(self.hasher.as_ref(), &group, &record_history)?;
        Ok(MeasureDecisionRecordStoredOperation {
            group,
            origin,
            record_history,
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
    material: MeasureDecisionRecordReady,
) -> Result<PreparedMeasureDecisionRecord, ApplicationError> {
    super::decision_wire::bounded(material.predecessors.len())?;
    super::decision_wire::bounded(material.result_sources.len())?;
    super::anchor_validation::shape(&material.anchor)?;
    evidence::limits(
        &material.record_history,
        1,
        command.outcome.affected_ids().len(),
    )?;
    if material.context.material().case_id != case_id {
        return Err(invalid("ready decision context belongs to another case"));
    }
    let support = admit_measure_decision_support(
        &command.values,
        std::slice::from_ref(&material.support_record),
        services.processor,
        services.limits,
        services.validator,
    )?;
    let selected = MeasureDecisionMaterialV2 {
        context: material.context.clone(),
        support,
        anchor: material.anchor.clone(),
        predecessors: material.predecessors.clone(),
        result_sources: material.result_sources.clone(),
    };
    let checked = prepare_measure_decision_with_record_history(
        services.hasher.as_ref(),
        actor,
        case_id,
        command,
        selected,
        &material.record_history,
    )?;
    Ok(PreparedMeasureDecisionRecord {
        checked,
        material,
        actor: actor.clone(),
        hasher: services.hasher,
    })
}
