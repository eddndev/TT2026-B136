use super::{
    decision_wire::{bounded, invalid},
    *,
};
use crate::{precautionary_hearings::source_inventory::SourceInventory, ApplicationError};
use domain::cases::CaseId;
use std::collections::{BTreeMap, BTreeSet};
type SourceId = [u8; 16];

pub(super) const MAX_GROUPS: usize = 256;
pub(super) const MAX_ROWS: usize = 8192;
type MemberKey = (SourceId, u32);

pub(super) struct GroupIndex<'a> {
    pub evidence: &'a MeasureHistoryEvidence,
    operations: BTreeMap<SourceId, usize>,
    members: BTreeMap<MemberKey, (usize, usize)>,
}

pub(crate) fn shape(group: &MeasureDecisionGroupCapture) -> Result<(), ApplicationError> {
    bounded(group.measures.len())?;
    bounded(group.review.results.len())?;
    bounded(group.review.material.result_sources.len())?;
    bounded(group.review.material.predecessors.len())?;
    bounded(group.substitutions.len())?;
    super::anchor_validation::shape(&group.review.material.anchor)?;
    super::anchor_validation::shape(&group.decision.anchor)?;
    for relationship in &group.substitutions {
        bounded(relationship.predecessors.len())?;
        bounded(relationship.successors.len())?;
    }
    Ok(())
}

pub(super) fn limits(
    evidence: &MeasureHistoryEvidence,
    extra_groups: usize,
    extra_rows: usize,
) -> Result<(), ApplicationError> {
    if evidence.groups.len() > MAX_GROUPS.saturating_sub(extra_groups) {
        return Err(invalid("incomplete measure history: group budget exceeded"));
    }
    let mut rows = extra_rows;
    for entry in &evidence.groups {
        rows = rows
            .checked_add(entry.capture.measures.len())
            .ok_or_else(|| invalid("history row overflow"))?;
        if rows > MAX_ROWS {
            return Err(invalid("incomplete measure history: row budget exceeded"));
        }
        shape(&entry.capture)?;
    }
    Ok(())
}

impl<'a> GroupIndex<'a> {
    pub fn new(
        case_id: CaseId,
        evidence: &'a MeasureHistoryEvidence,
    ) -> Result<Self, ApplicationError> {
        limits(evidence, 0, 0)?;
        let mut index = Self {
            evidence,
            operations: BTreeMap::new(),
            members: BTreeMap::new(),
        };
        let mut decisions = BTreeSet::new();
        for (position, entry) in evidence.groups.iter().enumerate() {
            let group = &entry.capture;
            if group.review.case_id != case_id
                || entry.origin != super::history_model::origin(group)
            {
                return Err(invalid("group origin or case differs"));
            }
            if index
                .operations
                .insert(*entry.origin.operation_id.as_uuid().as_bytes(), position)
                .is_some()
                || !decisions.insert(entry.origin.decision_id.as_uuid())
            {
                return Err(invalid("duplicate decision or operation in history"));
            }
            for (member, capture) in group.measures.iter().enumerate() {
                let key = (
                    *capture.result.id.as_uuid().as_bytes(),
                    capture.result.revision.get(),
                );
                if index.members.insert(key, (position, member)).is_some() {
                    return Err(invalid("ambiguous measure revision ownership"));
                }
            }
        }
        Ok(index)
    }

    pub fn selected(
        &self,
        reference: domain::precautionary_hearings::PrecautionaryMeasureRef,
    ) -> Result<(usize, usize), ApplicationError> {
        let found = self
            .members
            .get(&(
                *reference.id().as_uuid().as_bytes(),
                reference.revision().get(),
            ))
            .copied()
            .ok_or_else(|| invalid("missing exact owning group"))?;
        if self.evidence.groups[found.0].capture.measures[found.1].capture_digest
            != reference.digest()
        {
            return Err(invalid("selected measure digest differs"));
        }
        Ok(found)
    }

    pub fn dependency(&self, material: &OwnedMeasureMaterial) -> Result<usize, ApplicationError> {
        let group = *self
            .operations
            .get(material.owner.operation_id.as_uuid().as_bytes())
            .ok_or_else(|| invalid("missing predecessor owning group"))?;
        let entry = &self.evidence.groups[group].capture;
        if material.owner.decision_id != entry.review.command.decision_id
            || material.owner.group_digest != entry.capture_digest
        {
            return Err(invalid("predecessor owner differs"));
        }
        let (owner, row) = self.selected(super::effect_resolution::capture_reference(
            &material.capture,
        ))?;
        if owner != group || entry.measures[row] != material.capture {
            return Err(invalid(
                "predecessor is not an exact member of its owning group",
            ));
        }
        Ok(group)
    }
}

pub(crate) fn add_sources<'a>(
    inventory: &mut SourceInventory<'a>,
    review: &'a MeasureDecisionReview,
) -> Result<(), ApplicationError> {
    inventory.context(&review.material.context)?;
    if let Some(anchor) = &review.material.anchor {
        inventory.anchor(anchor)?;
    }
    inventory.support(&review.material.support)?;
    for result in &review.results {
        if let Some(projection) = &result.projection.supervisor {
            inventory.projection(&projection.overview, projection.snapshot.values_digest)?;
        }
        inventory.subject(&result.sources.subject)?;
        if let Some(source) = &result.sources.supervisor {
            inventory.participant(source)?;
        }
    }
    Ok(())
}
