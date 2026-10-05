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
        let capture = self.hearing_capture(reference)?.clone();
        let captures = self.prefix(reference.hearing_id.as_uuid(), reference.revision.get())?;
        let first = &captures[0];
        let first_proof = self.subclosure(first.review.resolved_values.review_targets())?;
        let origin = precautionary_hearing_origin_with_measure_history(hasher, first, &first_proof)
            .map_err(inconsistent)?;
        let refs: Vec<_> = captures
            .iter()
            .flat_map(|c| c.review.resolved_values.review_targets().iter().copied())
            .collect();
        let measure_history = self.subclosure(&refs)?;
        precautionary_hearing_history_with_measure_history_matches(
            hasher,
            &captures,
            &origin,
            &measure_history,
        )
        .map_err(inconsistent)?;
        Ok(PrecautionaryHearingStoredOperation {
            capture,
            history: PrecautionaryHearingHistoryEvidence {
                origin,
                captures,
                measure_history,
            },
        })
    }
    fn prefix(
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
        let mut groups: Vec<_> = self.groups.values().cloned().collect();
        if let Some(group) = group {
            if self
                .groups
                .contains_key(&group.origin.operation_id.as_uuid())
            {
                return Err(inconsistent("candidate group already owns history"));
            }
            groups.push(MeasureGroupEvidence {
                origin: group.origin.clone(),
                capture: group.group.clone(),
            });
        }
        let mut captures = self.hearings.clone();
        if let Some(hearing) = hearing {
            for capture in &hearing.history.captures {
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
        let mut prefixes: BTreeMap<Uuid, Vec<PrecautionaryHearingCapture>> = BTreeMap::new();
        for ((id, _), capture) in captures {
            prefixes.entry(id).or_default().push(capture);
        }
        let mut histories = Vec::new();
        for captures in prefixes.into_values() {
            let first = &captures[0];
            let origin = if let Some(candidate) =
                hearing.filter(|h| h.history.origin.hearing_id == first.review.command.hearing_id)
            {
                candidate.history.origin.clone()
            } else {
                let proof = self.subclosure(first.review.resolved_values.review_targets())?;
                precautionary_hearing_origin_with_measure_history(hasher, first, &proof)
                    .map_err(inconsistent)?
            };
            histories.push(MeasureAdministrativeHearingHistory { origin, captures });
        }
        validate_measure_dependency_inventory(
            hasher,
            case,
            &MeasureAdministrativeDependencyInventory {
                records: MeasureDecisionRecordHistoryEvidence {
                    records: MeasureRecordHistoryEvidence {
                        judicial: MeasureHistoryEvidence { groups },
                        administrative: Vec::new(),
                    },
                    decisions: Vec::new(),
                },
                hearings: histories,
            },
        )
        .map_err(inconsistent)
    }
}
