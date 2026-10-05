use super::{
    anchors, graph_discovery, history_budget::Budget, inconsistent, loaded_administrations,
    loaded_history, storage, LoadedMeasureHistory,
};
use application::{
    measure_corrections::MeasureAdministrativeEvidence, precautionary_hearings::*,
    precautionary_measures::*, ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    precautionary_hearings::*,
};
use postgres::{Row, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(crate) struct HearingProofRef {
    pub hearing_id: PrecautionaryHearingId,
    pub revision: PrecautionaryHearingRevision,
    pub capture_digest: Sha256Digest,
}
#[derive(Clone, Copy)]
pub(crate) enum HistoryRoot {
    Measure(PrecautionaryMeasureRef),
    Decision(domain::precautionary_measures::MeasureDecisionOperationId),
    Administrative(domain::precautionary_measures::MeasureCorrectionOperationId),
    Hearing(HearingProofRef),
}
#[derive(Default)]
pub(crate) struct HistoryReserve {
    pub groups: usize,
    pub members: usize,
    pub hearing_captures: usize,
    pub review_target_occurrences: usize,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key {
    Group(Uuid),
    Hearing(Uuid, u32),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OwnerFamily {
    JudicialV1,
    JudicialV2,
    Administrative,
}
pub(super) struct Node {
    pub row: Row,
    pub dependencies: BTreeSet<Key>,
    pub wire_parents: BTreeSet<Uuid>,
    pub targets: Vec<PrecautionaryMeasureRef>,
    pub family: Option<OwnerFamily>,
}

pub(crate) fn load_precautionary_history(
    tx: &mut Transaction<'_>,
    case: CaseId,
    roots: &[HistoryRoot],
    reserve: HistoryReserve,
    hasher: &dyn DocumentHasher,
) -> Result<LoadedMeasureHistory, ApplicationError> {
    let mut rows = graph_discovery::discover(
        tx,
        case,
        roots,
        Budget::new(reserve.groups, reserve.members),
        reserve,
    )?;
    let mut groups = BTreeMap::new();
    let mut decisions = BTreeMap::new();
    let mut administrations = BTreeMap::new();
    let mut parents = BTreeMap::new();
    let mut hearings: BTreeMap<(Uuid, u32), PrecautionaryHearingCapture> = BTreeMap::new();
    let mut hearing_origins = BTreeMap::new();
    let order = super::graph_order::order(
        rows.iter()
            .map(|(key, node)| (*key, node.dependencies.clone()))
            .collect(),
    )?;
    for key in order {
        let node = rows
            .remove(&key)
            .ok_or_else(|| inconsistent("durable graph node disappeared"))?;
        match key {
            Key::Group(op) if node.family == Some(OwnerFamily::Administrative) => {
                let proof = loaded_administrations::record_evidence_for(
                    &node.wire_parents,
                    &groups,
                    &administrations,
                    &decisions,
                    &parents,
                )?;
                let result = crate::measure_administrative_postgres::storage::reconstruct(
                    tx, &node.row, proof, hasher,
                )?;
                parents.insert(op, node.wire_parents);
                administrations.insert(
                    op,
                    MeasureAdministrativeEvidence {
                        origin: result.origin,
                        capture: result.capture,
                    },
                );
            }
            Key::Group(op) => {
                if !matches!(
                    node.family,
                    Some(OwnerFamily::JudicialV1 | OwnerFamily::JudicialV2)
                ) {
                    return Err(inconsistent("loaded owner family is absent"));
                }
                let reference = anchors::reference(&node.row)?;
                let anchor = match reference {
                    Some(MeasureDecisionAnchorRef::Precautionary {
                        hearing_id,
                        revision,
                        capture_digest,
                    }) => {
                        let capture = hearings
                            .get(&(hearing_id.as_uuid(), revision.get()))
                            .ok_or_else(|| inconsistent("selected anchor was not reconstructed"))?;
                        if capture.capture_digest != capture_digest {
                            return Err(inconsistent("anchor capture digest differs"));
                        }
                        Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
                            capture.clone(),
                        )))
                    }
                    _ => anchors::load(tx, case, &reference, hasher)?,
                };
                if node.family == Some(OwnerFamily::JudicialV1) {
                    let proof =
                        loaded_history::evidence_for(&node.wire_parents, &groups, &parents)?;
                    let result = storage::reconstruct(tx, &node.row, proof, anchor, hasher)?;
                    groups.insert(
                        op,
                        MeasureGroupEvidence {
                            origin: result.origin,
                            capture: result.group,
                        },
                    );
                } else {
                    let proof = loaded_administrations::record_evidence_for(
                        &node.wire_parents,
                        &groups,
                        &administrations,
                        &decisions,
                        &parents,
                    )?;
                    let result =
                        super::record_storage::reconstruct(tx, &node.row, proof, anchor, hasher)?;
                    decisions.insert(
                        op,
                        MeasureGroupEvidenceV2 {
                            origin: result.origin,
                            capture: result.group,
                        },
                    );
                }
                parents.insert(op, node.wire_parents);
            }
            Key::Hearing(id, revision) => {
                let proof = loaded_administrations::record_evidence_for(
                    &node.wire_parents,
                    &groups,
                    &administrations,
                    &decisions,
                    &parents,
                )?;
                let previous = revision.checked_sub(1).and_then(|r| hearings.get(&(id, r)));
                let capture =
                    if proof.records.administrative.is_empty() && proof.decisions.is_empty() {
                        crate::precautionary_hearing_postgres::decode::capture(
                            tx,
                            &node.row,
                            previous,
                            &proof.records.judicial,
                            hasher,
                        )?
                    } else {
                        crate::precautionary_hearing_postgres::decode::record_capture(
                            tx, &node.row, previous, &proof, hasher,
                        )?
                    };
                crate::precautionary_hearing_postgres::audit::verify(tx, &capture, hasher)?;
                if revision == 1 {
                    hearing_origins.insert(
                        id,
                        precautionary_hearing_origin_with_decision_history(
                            hasher, &capture, &proof,
                        )
                        .map_err(inconsistent)?,
                    );
                }
                hearings.insert((id, revision), capture);
            }
        }
    }
    let mut loaded = LoadedMeasureHistory::new(groups, decisions, administrations, parents)?;
    loaded.hearings = hearings;
    loaded.hearing_origins = hearing_origins;
    loaded.validate_forest(case, hasher, None, None)?;
    Ok(loaded)
}
