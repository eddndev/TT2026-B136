use super::*;
use crate::{precautionary_measures::MeasureHistoryEvidence, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher};
use std::collections::BTreeMap;

pub(super) fn invalid(message: &str) -> ApplicationError {
    PrecautionaryHearingError::StoredInconsistent(message.into()).into()
}

pub(super) fn clock(at: OffsetDateTime) -> Result<(), ApplicationError> {
    if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(invalid("service clock must use supported UTC"));
    }
    Ok(())
}

pub(super) fn history(
    hasher: &dyn DocumentHasher,
    evidence: &PrecautionaryHearingHistoryEvidence,
) -> Result<(), ApplicationError> {
    if evidence.captures.len() > 256 {
        return Err(PrecautionaryHearingError::IncompleteHistory.into());
    }
    precautionary_hearing_history_with_measure_history_matches(
        hasher,
        &evidence.captures,
        &evidence.origin,
        &evidence.measure_history,
    )
}

pub(super) fn operation(
    hasher: &dyn DocumentHasher,
    value: &PrecautionaryHearingStoredOperation,
) -> Result<(), ApplicationError> {
    history(hasher, &value.history)?;
    if value.history.captures.last() != Some(&value.capture) {
        return Err(invalid(
            "operation differs from the last capture of its original prefix",
        ));
    }
    Ok(())
}

pub(super) fn merge(
    a: &MeasureHistoryEvidence,
    b: &MeasureHistoryEvidence,
) -> Result<MeasureHistoryEvidence, ApplicationError> {
    if a.groups.len() > 256 || b.groups.len() > 256 {
        return Err(PrecautionaryHearingError::IncompleteHistory.into());
    }
    let mut groups = BTreeMap::new();
    for entry in a.groups.iter().chain(&b.groups) {
        let key = entry.origin.operation_id.as_uuid();
        if let Some(previous) = groups.insert(key, entry) {
            if previous != entry {
                return Err(invalid("shared group evidence differs"));
            }
        }
        if groups.len() > 256 {
            return Err(PrecautionaryHearingError::IncompleteHistory.into());
        }
    }
    let rows: usize = groups.values().map(|g| g.capture.measures.len()).sum();
    if rows > 8192 {
        return Err(PrecautionaryHearingError::IncompleteHistory.into());
    }
    Ok(MeasureHistoryEvidence {
        groups: groups.into_values().cloned().collect(),
    })
}
