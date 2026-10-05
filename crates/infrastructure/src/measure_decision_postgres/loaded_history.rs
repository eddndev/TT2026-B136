use super::inconsistent;
use application::{precautionary_measures::*, ApplicationError};
use domain::{crypto::Sha256Digest, precautionary_hearings::PrecautionaryMeasureRef};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub(crate) struct LoadedMeasureHistory {
    pub(super) groups: BTreeMap<Uuid, MeasureGroupEvidence>,
    parents: BTreeMap<Uuid, BTreeSet<Uuid>>,
    pub(super) hearings:
        BTreeMap<(Uuid, u32), application::precautionary_hearings::PrecautionaryHearingCapture>,
    members: BTreeMap<(Uuid, u32), (Sha256Digest, Uuid)>,
}

impl LoadedMeasureHistory {
    pub(super) fn new(
        groups: BTreeMap<Uuid, MeasureGroupEvidence>,
        parents: BTreeMap<Uuid, BTreeSet<Uuid>>,
    ) -> Result<Self, ApplicationError> {
        let mut members = BTreeMap::new();
        for (owner, group) in &groups {
            if *owner != group.origin.operation_id.as_uuid() || !parents.contains_key(owner) {
                return Err(inconsistent("loaded owner identity or dependencies differ"));
            }
            for capture in &group.capture.measures {
                let key = (capture.result.id.as_uuid(), capture.result.revision.get());
                if members
                    .insert(key, (capture.capture_digest, *owner))
                    .is_some()
                {
                    return Err(inconsistent("one measure revision has multiple owners"));
                }
            }
        }
        Ok(Self {
            groups,
            parents,
            hearings: BTreeMap::new(),
            members,
        })
    }

    pub(crate) fn subclosure(
        &self,
        refs: &[PrecautionaryMeasureRef],
    ) -> Result<MeasureHistoryEvidence, ApplicationError> {
        let roots = self.owner_ids(refs)?;
        evidence_for(&roots, &self.groups, &self.parents)
    }

    pub(super) fn owner_ids(
        &self,
        refs: &[PrecautionaryMeasureRef],
    ) -> Result<BTreeSet<Uuid>, ApplicationError> {
        let mut owners = BTreeSet::new();
        for reference in references(refs)? {
            let key = (reference.id().as_uuid(), reference.revision().get());
            let (digest, owner) = self
                .members
                .get(&key)
                .ok_or_else(|| inconsistent("target is absent from loaded owner evidence"))?;
            if *digest != reference.digest() {
                return Err(inconsistent(
                    "target digest differs from loaded owner evidence",
                ));
            }
            owners.insert(*owner);
        }
        Ok(owners)
    }

    pub(super) fn into_operation(
        mut self,
        operation: Uuid,
    ) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
        let roots = self
            .parents
            .get(&operation)
            .ok_or_else(|| inconsistent("selected owner was not reconstructed"))?;
        let measure_history = evidence_for(roots, &self.groups, &self.parents)?;
        let group = self
            .groups
            .remove(&operation)
            .ok_or_else(|| inconsistent("selected owner was not reconstructed"))?;
        Ok(MeasureDecisionStoredOperation {
            origin: group.origin,
            group: group.capture,
            measure_history,
        })
    }
}

pub(super) fn references(
    refs: &[PrecautionaryMeasureRef],
) -> Result<Vec<PrecautionaryMeasureRef>, ApplicationError> {
    if refs.len() > 8192 {
        return Err(inconsistent("measure target list exceeds 8192 references"));
    }
    let mut unique = BTreeMap::new();
    for reference in refs {
        let key = (reference.id().as_uuid(), reference.revision().get());
        if let Some(previous) = unique.insert(key, *reference) {
            if previous != *reference {
                return Err(inconsistent("one target revision has conflicting digests"));
            }
        }
    }
    Ok(unique.into_values().collect())
}

pub(super) fn evidence_for(
    roots: &BTreeSet<Uuid>,
    groups: &BTreeMap<Uuid, MeasureGroupEvidence>,
    parents: &BTreeMap<Uuid, BTreeSet<Uuid>>,
) -> Result<MeasureHistoryEvidence, ApplicationError> {
    let mut selected = BTreeSet::new();
    let mut pending: Vec<_> = roots.iter().copied().collect();
    while let Some(owner) = pending.pop() {
        if !selected.insert(owner) {
            continue;
        }
        let dependencies = parents
            .get(&owner)
            .ok_or_else(|| inconsistent("loaded owner dependencies are absent"))?;
        pending.extend(dependencies.iter().copied());
    }
    let groups = selected
        .into_iter()
        .map(|owner| {
            groups
                .get(&owner)
                .cloned()
                .ok_or_else(|| inconsistent("loaded ancestor owner is absent"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MeasureHistoryEvidence { groups })
}
