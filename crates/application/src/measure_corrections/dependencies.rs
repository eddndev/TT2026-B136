use super::{record_index::RecordIndex, wire::invalid, *};
use crate::{
    precautionary_hearings::{check_history_with_records, source_inventory::SourceInventory},
    precautionary_measures::MeasureDecisionAnchorMaterial,
    ApplicationError,
};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
};
use std::collections::{BTreeMap, BTreeSet};

/// Inspects direct uses in a validated supplied forest, without asserting durable completeness.
pub fn inspect_measure_administrative_dependencies(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    target: PrecautionaryMeasureRef,
    inventory: &MeasureAdministrativeDependencyInventory,
) -> Result<CheckedMeasureAdministrativeDependencies, ApplicationError> {
    bounds(inventory)?;
    let index = RecordIndex::new(case_id, (&inventory.records).into(), 0)?;
    index.selected(target)?;
    let additional = anchor_dependencies(&index, inventory)?;
    let mut sources = SourceInventory::default();
    let index = record_graph::validate_forest(hasher, case_id, index, &additional, &mut sources)?;
    for hearing in &inventory.hearings {
        check_history_with_records(
            hasher,
            case_id,
            &hearing.captures,
            &hearing.origin,
            &mut sources,
            &|r| index.view(index.selected(r)?),
        )?;
    }
    Ok(CheckedMeasureAdministrativeDependencies {
        case_id,
        target,
        dependants: dependency_report::collect(target, inventory),
    })
}

fn bounds(inventory: &MeasureAdministrativeDependencyInventory) -> Result<(), ApplicationError> {
    record_bounds::bounds((&inventory.records).into(), 0)?;
    if inventory.hearings.len() > 256 {
        return Err(invalid("hearing prefix count exceeds limit"));
    }
    let mut captures = 0usize;
    let mut targets = 0usize;
    for prefix in &inventory.hearings {
        captures = captures
            .checked_add(prefix.captures.len())
            .ok_or_else(|| invalid("hearing capture count overflow"))?;
        if prefix.captures.is_empty() || captures > 256 {
            return Err(invalid("hearing prefix capture budget exceeded"));
        }
        for capture in &prefix.captures {
            let review = &capture.review;
            let count = review.resolved_values.review_targets().len();
            if count > 32
                || review.sources.participants.len() > 32
                || review.participants.len() > 32
            {
                return Err(invalid("hearing material count exceeds limit"));
            }
            targets = targets
                .checked_add(count)
                .ok_or_else(|| invalid("hearing target count overflow"))?;
            if targets > 8192 {
                return Err(invalid("hearing target occurrence budget exceeded"));
            }
        }
    }
    Ok(())
}

fn anchor_dependencies(
    index: &RecordIndex<'_>,
    inventory: &MeasureAdministrativeDependencyInventory,
) -> Result<Vec<Vec<usize>>, ApplicationError> {
    let mut prefixes = BTreeMap::new();
    for prefix in &inventory.hearings {
        let first = &prefix.captures[0];
        let id = first.review.command.hearing_id.as_uuid();
        if prefixes.insert(id, prefix).is_some() {
            return Err(invalid("duplicate hearing prefix"));
        }
    }
    let anchors = index
        .evidence
        .judicial
        .groups
        .iter()
        .map(|g| &g.capture.review.material.anchor)
        .chain(
            index
                .evidence
                .decisions
                .iter()
                .map(|g| &g.capture.review.material.anchor),
        );
    let mut additional = vec![Vec::new(); index.owners()];
    for (owner, anchor) in anchors.enumerate() {
        let Some(MeasureDecisionAnchorMaterial::Precautionary(anchor)) = anchor else {
            continue;
        };
        let prefix = prefixes
            .get(&anchor.review.command.hearing_id.as_uuid())
            .ok_or_else(|| invalid("precautionary anchor lacks its hearing prefix"))?;
        let position = prefix
            .captures
            .iter()
            .position(|c| c.review.result_revision == anchor.review.result_revision)
            .ok_or_else(|| invalid("precautionary anchor revision lacks its prefix"))?;
        if prefix.captures[position] != **anchor {
            return Err(invalid(
                "precautionary anchor differs from full prefix capture",
            ));
        }
        let mut parents = BTreeSet::new();
        for capture in &prefix.captures[..=position] {
            for r in capture.review.resolved_values.review_targets() {
                parents.insert(index.owner(index.selected(*r)?));
            }
        }
        additional[owner] = parents.into_iter().collect();
    }
    Ok(additional)
}
