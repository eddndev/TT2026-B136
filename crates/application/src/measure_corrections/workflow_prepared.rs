use super::{workflow_evidence as evidence, *};
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    identity::Principal,
    precautionary_measures::MeasureDecisionRecordHistoryEvidence,
    ApplicationError,
};
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher, hearings::HearingSupportRef,
};
use std::sync::Arc;

/// Created only by the authorized service after complete evidence and support admission.
pub struct PreparedMeasureAdministrative {
    pub(super) checked: CheckedMeasureAdministrativeReview,
    pub(super) material: MeasureAdministrativeReady,
    pub(super) record_history: MeasureDecisionRecordHistoryEvidence,
    actor: Principal,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
impl PreparedMeasureAdministrative {
    pub fn review(&self) -> &MeasureAdministrativeReview {
        self.checked.review()
    }
    pub fn material(&self) -> &MeasureAdministrativeReady {
        &self.material
    }
    pub fn actor(&self) -> &Principal {
        &self.actor
    }
    pub fn into_operation(
        self,
        at: OffsetDateTime,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError> {
        let capture = self.checked.into_capture(self.hasher.as_ref(), at)?;
        let origin = super::capture::origin(&capture);
        Ok(MeasureAdministrativeStoredOperation {
            capture,
            origin,
            record_history: self.record_history,
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
    command: MeasureAdministrativeCommand,
    material: MeasureAdministrativeReady,
) -> Result<PreparedMeasureAdministrative, ApplicationError> {
    if material.target_head != command.target {
        return Err(MeasureAdministrativeError::StaleHead.into());
    }
    super::record_bounds::limits(
        (&material.dependency_inventory.records).into(),
        1,
        super::record_bounds::candidate_rows(&command),
    )?;
    let (index, mut sources) = super::dependencies::checked_forest(
        services.hasher.as_ref(),
        case_id,
        command.target,
        &material.dependency_inventory,
    )?;
    if !super::dependency_report::collect(command.target, &material.dependency_inventory).is_empty()
    {
        return Err(MeasureAdministrativeError::KnownDependants.into());
    }
    index.candidate(&command)?;
    let record_history = super::record_graph::extract_closure(&index, &command)?;
    let previous = index.view(index.selected(command.target)?)?;
    let checked = super::preparation::prepare_from_record_with_subject(
        services.hasher.as_ref(),
        actor,
        case_id,
        command,
        material.context.clone(),
        previous,
        material.replacement_subject.clone(),
    )?;
    super::record_history::add_sources(&mut sources, checked.review())?;
    let retained = &checked.review().support;
    let admitted = crate::documents::admit_exact_stage_support(
        HearingSupportRef::new(retained.reference, retained.digest),
        std::slice::from_ref(&material.support_record),
        services.processor,
        services.limits,
        services.validator,
    )?;
    if admitted != *retained {
        return Err(evidence::invalid(
            "admitted support differs from retained judicial evidence",
        ));
    }
    Ok(PreparedMeasureAdministrative {
        checked,
        material,
        record_history,
        actor: actor.clone(),
        hasher: services.hasher,
    })
}
