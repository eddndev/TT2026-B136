use super::{workflow_evidence::invalid, *};
use crate::{
    measure_corrections::{
        record_history_bounds, MeasureAdministrativeEvidence, MeasureRecordHistoryEvidence,
    },
    precautionary_measures::{
        MeasureDecisionRecordHistoryEvidence, MeasureGroupEvidence, MeasureGroupEvidenceV2,
        MeasureHistoryEvidence,
    },
    ApplicationError,
};
use domain::crypto::DocumentHasher;
use std::collections::{BTreeMap, BTreeSet};
type Id = [u8; 16];

pub(super) fn empty() -> MeasureDecisionRecordHistoryEvidence {
    MeasureDecisionRecordHistoryEvidence {
        records: MeasureRecordHistoryEvidence {
            judicial: MeasureHistoryEvidence { groups: vec![] },
            administrative: vec![],
        },
        decisions: vec![],
    }
}

pub(super) fn history_bounds(
    value: &PrecautionaryHearingRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    if value.captures.len() > 256 {
        return Err(PrecautionaryHearingError::IncompleteHistory.into());
    }
    record_history_bounds((&value.record_history).into())?;
    let mut targets = 0;
    for capture in &value.captures {
        super::record_evidence::shape(capture)?;
        targets += capture.review.resolved_values.review_targets().len();
    }
    if targets > 8192 {
        return Err(PrecautionaryHearingError::IncompleteHistory.into());
    }
    Ok(())
}

pub(super) fn ready_bounds(
    value: &PrecautionaryHearingRecordReady,
) -> Result<(), ApplicationError> {
    if value
        .selected_sources
        .as_ref()
        .is_some_and(|s| s.participants.len() > 32)
    {
        return Err(invalid("too many selected participants"));
    }
    record_history_bounds((&value.record_history).into())?;
    if let Some(history) = &value.history {
        history_bounds(history)?;
        if history.captures.len() >= 256 {
            return Err(PrecautionaryHearingError::IncompleteHistory.into());
        }
        owners(&history.record_history, &value.record_history)?;
    }
    Ok(())
}

pub(super) fn history(
    hasher: &dyn DocumentHasher,
    value: &PrecautionaryHearingRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    history_bounds(value)?;
    precautionary_hearing_history_with_decision_history_matches(
        hasher,
        &value.captures,
        &value.origin,
        &value.record_history,
    )
}

pub(super) fn operation(
    hasher: &dyn DocumentHasher,
    value: &PrecautionaryHearingRecordStoredOperation,
) -> Result<(), ApplicationError> {
    super::record_evidence::shape(&value.capture)?;
    history(hasher, &value.history)?;
    if value.history.captures.last() != Some(&value.capture) {
        return Err(invalid(
            "operation differs from the last capture of its original prefix",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner<'a> {
    Judicial(&'a MeasureGroupEvidence),
    Administrative(&'a MeasureAdministrativeEvidence),
    Decision(&'a MeasureGroupEvidenceV2),
}
impl Owner<'_> {
    fn rows(self) -> usize {
        match self {
            Self::Judicial(g) => g.capture.measures.len(),
            Self::Administrative(a) => a.capture.records.len(),
            Self::Decision(g) => g.capture.measures.len(),
        }
    }
}

fn owners<'a>(
    a: &'a MeasureDecisionRecordHistoryEvidence,
    b: &'a MeasureDecisionRecordHistoryEvidence,
) -> Result<BTreeMap<Id, Owner<'a>>, ApplicationError> {
    let mut merged = BTreeMap::new();
    for evidence in [a, b] {
        record_history_bounds(evidence.into())?;
        let mut unique = BTreeSet::new();
        let entries = evidence
            .records
            .judicial
            .groups
            .iter()
            .map(|g| {
                (
                    *g.origin.operation_id.as_uuid().as_bytes(),
                    Owner::Judicial(g),
                )
            })
            .chain(evidence.records.administrative.iter().map(|a| {
                (
                    *a.origin.operation_id.as_uuid().as_bytes(),
                    Owner::Administrative(a),
                )
            }))
            .chain(evidence.decisions.iter().map(|g| {
                (
                    *g.origin.operation_id.as_uuid().as_bytes(),
                    Owner::Decision(g),
                )
            }));
        for (key, entry) in entries {
            if !unique.insert(key) {
                return Err(invalid("operation identity recurs within record evidence"));
            }
            if let Some(previous) = merged.insert(key, entry) {
                if previous != entry {
                    return Err(invalid("shared record owner or family differs"));
                }
            }
            if merged.len() > 256 {
                return Err(PrecautionaryHearingError::IncompleteHistory.into());
            }
        }
    }
    if merged.values().map(|owner| owner.rows()).sum::<usize>() > 8192 {
        return Err(PrecautionaryHearingError::IncompleteHistory.into());
    }
    Ok(merged)
}

pub(super) fn merge(
    a: &MeasureDecisionRecordHistoryEvidence,
    b: &MeasureDecisionRecordHistoryEvidence,
) -> Result<MeasureDecisionRecordHistoryEvidence, ApplicationError> {
    let owners = owners(a, b)?;
    let mut result = empty();
    for owner in owners.into_values() {
        match owner {
            Owner::Judicial(g) => result.records.judicial.groups.push(g.clone()),
            Owner::Administrative(a) => result.records.administrative.push(a.clone()),
            Owner::Decision(g) => result.decisions.push(g.clone()),
        }
    }
    Ok(result)
}
