use super::{
    inconsistent, loaded_administrations::record_evidence_for, HearingProofRef,
    LoadedMeasureHistory,
};
use application::{precautionary_hearings::*, precautionary_measures::*, ApplicationError};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_measures::MeasureDecisionOperationId,
};

impl LoadedMeasureHistory {
    pub(crate) fn into_record_operation(
        mut self,
        operation: MeasureDecisionOperationId,
    ) -> Result<MeasureDecisionRecordStoredOperation, ApplicationError> {
        let id = operation.as_uuid();
        let roots = self
            .parents
            .get(&id)
            .ok_or_else(|| inconsistent("selected V2 owner dependencies are absent"))?;
        let record_history = record_evidence_for(
            roots,
            &self.groups,
            &self.administrations,
            &self.decisions,
            &self.parents,
        )?;
        let group = self
            .decisions
            .remove(&id)
            .ok_or_else(|| inconsistent("selected owner is not a reconstructed V2 decision"))?;
        Ok(MeasureDecisionRecordStoredOperation {
            origin: group.origin,
            group: group.capture,
            record_history,
        })
    }

    pub(crate) fn hearing_record_operation(
        &self,
        reference: HearingProofRef,
        hasher: &dyn DocumentHasher,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
        let capture = self.hearing_capture(reference)?.clone();
        let captures = self.prefix(reference.hearing_id.as_uuid(), reference.revision.get())?;
        let origin = self
            .hearing_origins
            .get(&reference.hearing_id.as_uuid())
            .cloned()
            .ok_or_else(|| inconsistent("selected hearing origin is absent"))?;
        let refs: Vec<_> = captures
            .iter()
            .flat_map(|capture| {
                capture
                    .review
                    .resolved_values
                    .review_targets()
                    .iter()
                    .copied()
            })
            .collect();
        let record_history = self.record_subclosure(&refs)?;
        precautionary_hearing_history_with_decision_history_matches(
            hasher,
            &captures,
            &origin,
            &record_history,
        )
        .map_err(inconsistent)?;
        Ok(PrecautionaryHearingRecordStoredOperation {
            capture,
            history: PrecautionaryHearingRecordHistoryEvidence {
                origin,
                captures,
                record_history,
            },
        })
    }

    pub(crate) fn validate_record_candidate(
        &self,
        case: CaseId,
        hasher: &dyn DocumentHasher,
        candidate: &MeasureDecisionRecordStoredOperation,
    ) -> Result<(), ApplicationError> {
        let operation = candidate.origin.operation_id.as_uuid();
        if self.groups.contains_key(&operation)
            || self.decisions.contains_key(&operation)
            || self.administrations.contains_key(&operation)
        {
            return Err(inconsistent("candidate V2 group already owns history"));
        }
        let mut inventory = self.dependency_inventory()?;
        inventory.records.decisions.push(MeasureGroupEvidenceV2 {
            origin: candidate.origin.clone(),
            capture: candidate.group.clone(),
        });
        self.check_forest(case, hasher, inventory, None)
    }

    pub(crate) fn validate_hearing_record_candidate(
        &self,
        case: CaseId,
        hasher: &dyn DocumentHasher,
        candidate: &PrecautionaryHearingRecordStoredOperation,
    ) -> Result<(), ApplicationError> {
        self.check_forest(
            case,
            hasher,
            self.dependency_inventory()?,
            Some((&candidate.history.origin, &candidate.history.captures)),
        )
    }
}
