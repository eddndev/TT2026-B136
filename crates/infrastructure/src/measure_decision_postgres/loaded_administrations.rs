use super::{inconsistent, loaded_history::selected_owners, LoadedMeasureHistory};
use application::{measure_corrections::*, precautionary_measures::*, ApplicationError};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::MeasureCorrectionOperationId,
};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

impl LoadedMeasureHistory {
    pub(crate) fn record_subclosure(
        &self,
        refs: &[PrecautionaryMeasureRef],
    ) -> Result<MeasureDecisionRecordHistoryEvidence, ApplicationError> {
        record_evidence_for(
            &self.owner_ids(refs)?,
            &self.groups,
            &self.administrations,
            &self.parents,
        )
    }

    pub(crate) fn into_administrative_operation(
        mut self,
        operation: MeasureCorrectionOperationId,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError> {
        let id = operation.as_uuid();
        let roots = self
            .parents
            .get(&id)
            .ok_or_else(|| inconsistent("selected administrative dependencies are absent"))?;
        let record_history =
            record_evidence_for(roots, &self.groups, &self.administrations, &self.parents)?;
        let entry = self
            .administrations
            .remove(&id)
            .ok_or_else(|| inconsistent("selected administrative owner is absent"))?;
        Ok(MeasureAdministrativeStoredOperation {
            capture: entry.capture,
            origin: entry.origin,
            record_history,
        })
    }

    pub(crate) fn dependency_inventory(
        &self,
    ) -> Result<MeasureAdministrativeDependencyInventory, ApplicationError> {
        Ok(MeasureAdministrativeDependencyInventory {
            records: MeasureDecisionRecordHistoryEvidence {
                records: MeasureRecordHistoryEvidence {
                    judicial: MeasureHistoryEvidence {
                        groups: self.groups.values().cloned().collect(),
                    },
                    administrative: self.administrations.values().cloned().collect(),
                },
                decisions: Vec::new(),
            },
            hearings: self.hearing_histories(&self.hearings, None)?,
        })
    }

    pub(crate) fn validate_administrative_candidate(
        &self,
        case: CaseId,
        hasher: &dyn DocumentHasher,
        candidate: &MeasureAdministrativeStoredOperation,
    ) -> Result<(), ApplicationError> {
        let operation = candidate.origin.operation_id.as_uuid();
        if self.groups.contains_key(&operation) || self.administrations.contains_key(&operation) {
            return Err(inconsistent(
                "candidate administrative operation already owns history",
            ));
        }
        let mut inventory = self.dependency_inventory()?;
        inventory
            .records
            .records
            .administrative
            .push(MeasureAdministrativeEvidence {
                origin: candidate.origin.clone(),
                capture: candidate.capture.clone(),
            });
        validate_measure_dependency_inventory(hasher, case, &inventory).map_err(inconsistent)
    }
}

pub(super) fn record_evidence_for(
    roots: &BTreeSet<Uuid>,
    groups: &BTreeMap<Uuid, MeasureGroupEvidence>,
    administrations: &BTreeMap<Uuid, MeasureAdministrativeEvidence>,
    parents: &BTreeMap<Uuid, BTreeSet<Uuid>>,
) -> Result<MeasureDecisionRecordHistoryEvidence, ApplicationError> {
    let mut judicial = Vec::new();
    let mut administrative = Vec::new();
    for owner in selected_owners(roots, parents)? {
        match (groups.get(&owner), administrations.get(&owner)) {
            (Some(group), None) => judicial.push(group.clone()),
            (None, Some(capture)) => administrative.push(capture.clone()),
            _ => {
                return Err(inconsistent(
                    "loaded ancestor has missing or conflicting owner family",
                ))
            }
        }
    }
    Ok(MeasureDecisionRecordHistoryEvidence {
        records: MeasureRecordHistoryEvidence {
            judicial: MeasureHistoryEvidence { groups: judicial },
            administrative,
        },
        decisions: Vec::new(),
    })
}
