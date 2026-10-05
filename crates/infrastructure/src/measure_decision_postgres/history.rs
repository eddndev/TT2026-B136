use super::{history_budget::Budget, inconsistent, port, storage};
use application::{precautionary_measures::*, ApplicationError};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::*,
};
use postgres::{Row, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub(super) fn selections(outcome: &MeasureDecisionOutcome) -> Vec<PrecautionaryMeasureRef> {
    let mut result = Vec::new();
    for effect in outcome.changes().unwrap_or(&[]) {
        match effect {
            MeasureEffect::Impose(_) => {}
            MeasureEffect::Confirm { previous }
            | MeasureEffect::Modify { previous, .. }
            | MeasureEffect::Revoke { previous }
            | MeasureEffect::Cease { previous } => result.push(*previous),
            MeasureEffect::Substitute { predecessors, .. } => {
                result.extend_from_slice(predecessors)
            }
        }
    }
    result.sort_by_key(|r| r.id().as_uuid());
    result
}
pub(super) fn predecessors(
    refs: &[PrecautionaryMeasureRef],
    evidence: &MeasureHistoryEvidence,
) -> Result<Vec<OwnedMeasureMaterial>, ApplicationError> {
    let mut result = Vec::with_capacity(refs.len());
    for reference in refs {
        let mut found = None;
        for group in &evidence.groups {
            for capture in &group.capture.measures {
                if capture.result.id == reference.id()
                    && capture.result.revision == reference.revision()
                {
                    if found.is_some() || capture.capture_digest != reference.digest() {
                        return Err(inconsistent("selected measure ownership differs"));
                    }
                    found = Some(OwnedMeasureMaterial {
                        owner: MeasureGroupRef {
                            operation_id: group.origin.operation_id,
                            decision_id: group.origin.decision_id,
                            group_digest: group.origin.group_digest,
                        },
                        capture: capture.clone(),
                    });
                }
            }
        }
        result.push(found.ok_or_else(|| inconsistent("selected measure has no complete owner"))?);
    }
    result.sort_by_key(|m| m.capture.result.id.as_uuid());
    Ok(result)
}
fn owner(
    tx: &mut Transaction<'_>,
    case: CaseId,
    reference: PrecautionaryMeasureRef,
) -> Result<Uuid, ApplicationError> {
    tx.query_opt("SELECT owner_operation FROM case_measure_revisions WHERE case_id=$1 AND measure_id=$2 AND revision=$3 AND capture_digest=$4 AND family='m1'",&[&case.as_uuid(),&reference.id().as_uuid(),&i64::from(reference.revision().get()),&reference.digest().as_bytes().as_slice()]).map_err(port)?.map(|r|r.get(0)).ok_or_else(||inconsistent("exact predecessor row is absent or inconsistent"))
}
pub(super) fn candidate(
    tx: &mut Transaction<'_>,
    case: CaseId,
    outcome: &MeasureDecisionOutcome,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureHistoryEvidence, ApplicationError> {
    let refs = selections(outcome);
    let roots = refs
        .iter()
        .map(|r| owner(tx, case, *r))
        .collect::<Result<Vec<_>, _>>()?;
    let groups = load(
        tx,
        case,
        roots,
        Budget::new(1, outcome.affected_ids().len()),
        hasher,
    )?
    .into_values()
    .map(|g| MeasureGroupEvidence {
        origin: g.origin,
        capture: g.group,
    })
    .collect();
    let evidence = MeasureHistoryEvidence { groups };
    predecessors(&refs, &evidence)?;
    Ok(evidence)
}
pub(super) fn operation(
    tx: &mut Transaction<'_>,
    case: CaseId,
    op: MeasureDecisionOperationId,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
    load(tx, case, vec![op.as_uuid()], Budget::new(0, 0), hasher)?
        .remove(&op.as_uuid())
        .ok_or_else(|| inconsistent("selected owner was not reconstructed"))
}
fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    mut pending: Vec<Uuid>,
    mut budget: Budget,
    hasher: &dyn DocumentHasher,
) -> Result<BTreeMap<Uuid, MeasureDecisionStoredOperation>, ApplicationError> {
    let mut rows: BTreeMap<Uuid, (Row, BTreeSet<Uuid>)> = BTreeMap::new();
    while let Some(op) = pending.pop() {
        if rows.contains_key(&op) {
            continue;
        }
        let count:i64=tx.query_one("SELECT count(*) FROM (SELECT 1 FROM case_measure_revisions WHERE owner_operation=$1 LIMIT 33) bounded",&[&op]).map_err(port)?.get(0);
        if !(0..=32).contains(&count) {
            return Err(inconsistent("measure owner exceeds 32 members"));
        }
        budget.admit(count as usize)?;
        let row = storage::raw(tx, case, MeasureDecisionOperationId::from_uuid(op))?;
        let outcome = crate::measure_decision_codec::outcome(
            &row.get::<_, Vec<u8>>("outcome_canonical"),
            &row.get("outcome_view"),
        )?;
        let parents = selections(&outcome)
            .iter()
            .map(|r| owner(tx, case, *r))
            .collect::<Result<BTreeSet<_>, _>>()?;
        pending.extend(parents.iter().copied());
        rows.insert(op, (row, parents));
    }
    let mut built: BTreeMap<Uuid, MeasureDecisionStoredOperation> = BTreeMap::new();
    while !rows.is_empty() {
        let ready = rows
            .iter()
            .find(|(_, (_, deps))| deps.iter().all(|id| built.contains_key(id)))
            .map(|(id, _)| *id)
            .ok_or_else(|| inconsistent("measure owner graph contains a cycle"))?;
        let (row, deps) = rows
            .remove(&ready)
            .ok_or_else(|| inconsistent("measure owner disappeared"))?;
        let mut ancestry = BTreeMap::new();
        for parent in deps {
            let stored = &built[&parent];
            for older in &stored.measure_history.groups {
                ancestry.insert(older.origin.operation_id.as_uuid(), older.clone());
            }
            ancestry.insert(
                parent,
                MeasureGroupEvidence {
                    origin: stored.origin.clone(),
                    capture: stored.group.clone(),
                },
            );
        }
        let result = storage::reconstruct(
            tx,
            &row,
            MeasureHistoryEvidence {
                groups: ancestry.into_values().collect(),
            },
            hasher,
        )?;
        built.insert(ready, result);
    }
    Ok(built)
}
