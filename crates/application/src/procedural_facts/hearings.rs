use super::{
    FactHearingRef, FactHearingSourceSnapshot, FactHearingView, FactSourceSelection,
    ProceduralFactError,
};
use crate::{
    hearing_results::{hearing_result_snapshot_receipt_matches, HearingResultSnapshot},
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};

/// Readable fields and captured digests derived from one exact result selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactHearingProjection {
    pub snapshot: FactHearingSourceSnapshot,
    pub view: FactHearingView,
}

/// Verifies the exact result union once, then projects each agreement selection.
/// Withdrawn revisions remain historical sources. This neither authorizes access
/// nor admits documents or recursively resolves sources recorded in these values.
pub fn resolve_fact_hearings(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selection: &FactSourceSelection,
    material: &[HearingResultSnapshot],
) -> Result<Vec<FactHearingProjection>, ApplicationError> {
    let selected = selection.hearing_results();
    let mut expected = selected.iter().map(reference_key).collect::<Vec<_>>();
    expected.sort_unstable();
    expected.dedup();
    if selected.len() > 2 || material.len() > 2 || material.len() != expected.len() {
        return Err(inconsistent(
            "hearing result material count differs from exact selection",
        ));
    }
    let mut ordered = material.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|snapshot| snapshot_key(snapshot));
    if ordered
        .windows(2)
        .any(|pair| snapshot_key(pair[0]) == snapshot_key(pair[1]))
    {
        return Err(inconsistent(
            "hearing result material repeats an exact revision",
        ));
    }
    for (snapshot, key) in ordered.iter().zip(expected) {
        if snapshot.case_id != case_id || snapshot_key(snapshot) != key {
            return Err(inconsistent(
                "hearing result material scope or exact reference differs",
            ));
        }
        hearing_result_snapshot_receipt_matches(hasher, snapshot).map_err(|_| {
            inconsistent("hearing result values or reconstructed receipt are inconsistent")
        })?;
    }
    selected
        .iter()
        .map(|reference| {
            let snapshot = ordered
                .iter()
                .find(|snapshot| snapshot_key(snapshot) == reference_key(reference))
                .ok_or_else(|| inconsistent("selected hearing result revision is missing"))?;
            project(case_id, *reference, snapshot)
        })
        .collect()
}

fn project(
    case_id: CaseId,
    reference: FactHearingRef,
    source: &HearingResultSnapshot,
) -> Result<FactHearingProjection, ApplicationError> {
    let agreement = reference
        .agreement_id
        .map(|id| {
            source
                .values
                .agreements()
                .iter()
                .find(|agreement| agreement.id() == id)
                .cloned()
                .ok_or_else(|| {
                    inconsistent("selected agreement is absent from the exact result revision")
                })
        })
        .transpose()?;
    Ok(FactHearingProjection {
        snapshot: FactHearingSourceSnapshot {
            case_id,
            reference,
            values_digest: source.values_digest,
            submission_digest: source.receipt.submission_digest,
            status: source.status,
        },
        view: FactHearingView {
            reference,
            occurrence: source.values.occurrence(),
            event_time: source.values.event_time(),
            summary: source.values.summary().clone(),
            agreement,
        },
    })
}

fn reference_key(reference: &FactHearingRef) -> ([u8; 16], [u8; 16], u32) {
    (
        *reference.hearing_id.as_uuid().as_bytes(),
        *reference.result_id.as_uuid().as_bytes(),
        reference.revision.get(),
    )
}
fn snapshot_key(snapshot: &HearingResultSnapshot) -> ([u8; 16], [u8; 16], u32) {
    (
        *snapshot.hearing_id.as_uuid().as_bytes(),
        *snapshot.id.as_uuid().as_bytes(),
        snapshot.revision.get(),
    )
}
fn inconsistent(message: &str) -> ApplicationError {
    ProceduralFactError::StoredInconsistent(message.into()).into()
}
