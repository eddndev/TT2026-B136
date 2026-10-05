use super::{
    anchors, audit, graph::*, graph_budget::HearingBudget, history, history_budget::Budget,
    inconsistent, port, storage,
};
use application::{precautionary_measures::*, ApplicationError};
use domain::{
    cases::CaseId, crypto::Sha256Digest, precautionary_hearings::*, precautionary_measures::*,
};
use postgres::Transaction;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

fn hearing_key(reference: HearingProofRef) -> Key {
    Key::Hearing(reference.hearing_id.as_uuid(), reference.revision.get())
}
fn claim(
    claims: &mut BTreeMap<Key, Sha256Digest>,
    reference: HearingProofRef,
) -> Result<Key, ApplicationError> {
    let key = hearing_key(reference);
    if claims
        .insert(key, reference.capture_digest)
        .is_some_and(|old| old != reference.capture_digest)
    {
        return Err(inconsistent(
            "one hearing revision has conflicting digest selections",
        ));
    }
    Ok(key)
}
fn owners(
    tx: &mut Transaction<'_>,
    case: CaseId,
    refs: &[PrecautionaryMeasureRef],
) -> Result<BTreeSet<Uuid>, ApplicationError> {
    refs.iter().map(|r| history::owner(tx, case, *r)).collect()
}

pub(super) fn discover(
    tx: &mut Transaction<'_>,
    case: CaseId,
    roots: &[HistoryRoot],
    mut budget: Budget,
    reserve: HistoryReserve,
) -> Result<BTreeMap<Key, Node>, ApplicationError> {
    if roots.len() > 8193 {
        return Err(inconsistent("durable proof root count exceeds limit"));
    }
    let mut hearing_budget =
        HearingBudget::new(reserve.hearing_captures, reserve.review_target_occurrences);
    let mut pending = Vec::new();
    let mut claims = BTreeMap::new();
    let mut rows: BTreeMap<Key, Node> = BTreeMap::new();
    for root in roots {
        pending.push(match *root {
            HistoryRoot::Measure(r) => Key::Group(history::owner(tx, case, r)?),
            HistoryRoot::Decision(op) => Key::Group(op.as_uuid()),
            HistoryRoot::Hearing(r) => claim(&mut claims, r)?,
        });
    }
    let mut checked_groups = false;
    while let Some(key) = pending.pop() {
        if rows.contains_key(&key) {
            continue;
        }
        match key {
            Key::Group(op) => {
                if !checked_groups {
                    audit::inventory_intact(tx)?;
                    checked_groups = true;
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
                let wire_parents = owners(tx, case, &history::selections(&outcome))?;
                let mut dependencies: BTreeSet<_> =
                    wire_parents.iter().copied().map(Key::Group).collect();
                if let Some(MeasureDecisionAnchorRef::Precautionary {
                    hearing_id,
                    revision,
                    capture_digest,
                }) = anchors::reference(&row)?
                {
                    dependencies.insert(claim(
                        &mut claims,
                        HearingProofRef {
                            hearing_id,
                            revision,
                            capture_digest,
                        },
                    )?);
                }
                pending.extend(dependencies.iter().copied());
                rows.insert(
                    key,
                    Node {
                        row,
                        dependencies,
                        wire_parents,
                        targets: Vec::new(),
                    },
                );
            }
            Key::Hearing(id, revision) => {
                let counts =
                    super::graph_prefix::admit(tx, case, id, revision, &mut hearing_budget)?;
                let prefix = crate::precautionary_hearing_postgres::storage::prefix(
                    tx,
                    case,
                    PrecautionaryHearingId::from_uuid(id),
                    Some(PrecautionaryHearingRevision::new(revision).map_err(inconsistent)?),
                )?;
                let mut previous_targets = Vec::new();
                for (index, (row, targets)) in
                    prefix.rows.into_iter().zip(prefix.targets).enumerate()
                {
                    let revision = index as u32 + 1;
                    let current = Key::Hearing(id, revision);
                    if counts.get(&revision) != Some(&targets.len()) {
                        return Err(inconsistent(
                            "hearing target count differs from bounded preflight",
                        ));
                    }
                    if let std::collections::btree_map::Entry::Vacant(entry) = rows.entry(current) {
                        let refs = super::loaded_history::references(
                            &[targets.as_slice(), previous_targets.as_slice()].concat(),
                        )?;
                        let wire_parents = owners(tx, case, &refs)?;
                        let mut dependencies: BTreeSet<_> =
                            wire_parents.iter().copied().map(Key::Group).collect();
                        if revision > 1 {
                            dependencies.insert(Key::Hearing(id, revision - 1));
                        }
                        pending.extend(dependencies.iter().copied());
                        entry.insert(Node {
                            row,
                            dependencies,
                            wire_parents,
                            targets: targets.clone(),
                        });
                    }
                    previous_targets = targets;
                }
            }
        }
    }
    for (key, digest) in claims {
        let row = &rows
            .get(&key)
            .ok_or_else(|| inconsistent("selected hearing capture is absent"))?
            .row;
        if super::decode::digest(row.get("capture_digest"))? != digest {
            return Err(inconsistent("selected hearing capture digest differs"));
        }
    }
    let additions: Vec<_> = rows
        .iter()
        .filter_map(|(key, node)| match key {
            Key::Group(_) => Some((|| {
                let Some(MeasureDecisionAnchorRef::Precautionary {
                    hearing_id,
                    revision,
                    ..
                }) = anchors::reference(&node.row)?
                else {
                    return Ok(None);
                };
                let hearing = rows
                    .get(&Key::Hearing(hearing_id.as_uuid(), revision.get()))
                    .ok_or_else(|| inconsistent("anchor node is absent"))?;
                Ok(Some((*key, owners(tx, case, &hearing.targets)?)))
            })()),
            _ => None,
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?
        .into_iter()
        .flatten()
        .collect();
    for (key, parents) in additions {
        rows.get_mut(&key)
            .ok_or_else(|| inconsistent("group node is absent"))?
            .wire_parents
            .extend(parents);
    }
    Ok(rows)
}
