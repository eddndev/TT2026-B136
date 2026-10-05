use super::{graph::HearingProofRef, inconsistent, LoadedMeasureHistory};
use application::{
    measure_corrections::*, precautionary_hearings::*, precautionary_measures::*, ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};
use std::collections::BTreeMap;
use uuid::Uuid;

impl LoadedMeasureHistory {
    pub(crate) fn hearing_capture(
        &self,
        reference: HearingProofRef,
    ) -> Result<&PrecautionaryHearingCapture, ApplicationError> {
        let capture = self
            .hearings
            .get(&(reference.hearing_id.as_uuid(), reference.revision.get()))
            .ok_or_else(|| inconsistent("exact hearing capture is absent from loaded proof"))?;
        if capture.capture_digest != reference.capture_digest {
            return Err(inconsistent("loaded hearing capture digest differs"));
        }
        Ok(capture)
    }
    pub(crate) fn hearing_operation(
        &self,
        reference: HearingProofRef,
        hasher: &dyn DocumentHasher,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
        let operation = self.hearing_record_operation(reference, hasher)?;
        if !operation
            .history
            .record_history
            .records
            .administrative
            .is_empty()
            || !operation.history.record_history.decisions.is_empty()
        {
            return Err(inconsistent(
                "legacy hearing proof requires only V1 judicial owners",
            ));
        }
        Ok(PrecautionaryHearingStoredOperation {
            capture: operation.capture,
            history: PrecautionaryHearingHistoryEvidence {
                origin: operation.history.origin,
                captures: operation.history.captures,
                measure_history: operation.history.record_history.records.judicial,
            },
        })
    }
    pub(super) fn prefix(
        &self,
        id: Uuid,
        revision: u32,
    ) -> Result<Vec<PrecautionaryHearingCapture>, ApplicationError> {
        (1..=revision)
            .map(|r| {
                self.hearings
                    .get(&(id, r))
                    .cloned()
                    .ok_or_else(|| inconsistent("loaded hearing prefix has a gap"))
            })
            .collect()
    }
    pub(crate) fn validate_forest(
        &self,
        case: CaseId,
        hasher: &dyn DocumentHasher,
        group: Option<&MeasureDecisionStoredOperation>,
        hearing: Option<&PrecautionaryHearingStoredOperation>,
    ) -> Result<(), ApplicationError> {
        let mut inventory = self.dependency_inventory()?;
        if let Some(group) = group {
            if self
                .groups
                .contains_key(&group.origin.operation_id.as_uuid())
                || self
                    .administrations
                    .contains_key(&group.origin.operation_id.as_uuid())
                || self
                    .decisions
                    .contains_key(&group.origin.operation_id.as_uuid())
            {
                return Err(inconsistent("candidate group already owns history"));
            }
            inventory
                .records
                .records
                .judicial
                .groups
                .push(MeasureGroupEvidence {
                    origin: group.origin.clone(),
                    capture: group.group.clone(),
                });
        }
        self.check_forest(
            case,
            hasher,
            inventory,
            hearing.map(|h| (&h.history.origin, h.history.captures.as_slice())),
        )
    }

    pub(super) fn check_forest(
        &self,
        case: CaseId,
        hasher: &dyn DocumentHasher,
        mut inventory: MeasureAdministrativeDependencyInventory,
        hearing: Option<(&PrecautionaryHearingOrigin, &[PrecautionaryHearingCapture])>,
    ) -> Result<(), ApplicationError> {
        let mut captures = self.hearings.clone();
        if let Some((_, prefix)) = hearing {
            for capture in prefix {
                let key = (
                    capture.review.command.hearing_id.as_uuid(),
                    capture.review.result_revision.get(),
                );
                if captures
                    .insert(key, capture.clone())
                    .is_some_and(|old| old != *capture)
                {
                    return Err(inconsistent(
                        "candidate hearing prefix differs from durable history",
                    ));
                }
            }
        }
        inventory.hearings = self.hearing_histories(&captures, hearing.map(|h| h.0))?;
        validate_measure_dependency_inventory(hasher, case, &inventory).map_err(inconsistent)
    }

    pub(super) fn hearing_histories(
        &self,
        captures: &BTreeMap<(Uuid, u32), PrecautionaryHearingCapture>,
        candidate: Option<&PrecautionaryHearingOrigin>,
    ) -> Result<Vec<MeasureAdministrativeHearingHistory>, ApplicationError> {
        let mut prefixes: BTreeMap<Uuid, Vec<PrecautionaryHearingCapture>> = BTreeMap::new();
        for ((id, _), capture) in captures {
            prefixes.entry(*id).or_default().push(capture.clone());
        }
        let mut histories = Vec::new();
        for captures in prefixes.into_values() {
            let first = &captures[0];
            let origin = if let Some(candidate) =
                candidate.filter(|h| h.hearing_id == first.review.command.hearing_id)
            {
                candidate.clone()
            } else {
                self.hearing_origins
                    .get(&first.review.command.hearing_id.as_uuid())
                    .cloned()
                    .ok_or_else(|| inconsistent("loaded hearing origin is absent"))?
            };
            histories.push(MeasureAdministrativeHearingHistory { origin, captures });
        }
        Ok(histories)
    }
}
