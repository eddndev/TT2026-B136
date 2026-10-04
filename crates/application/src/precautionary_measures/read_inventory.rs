use super::{workflow_evidence::invalid, *};
use crate::{precautionary_hearings::source_inventory::SourceInventory, ApplicationError};
use std::collections::BTreeMap;
type SourceId = [u8; 16];

/// Independently valid groups must agree on shared sources and immutable owners.
#[derive(Default)]
pub(super) struct ReadInventory<'a> {
    sources: SourceInventory<'a>,
    groups: BTreeMap<SourceId, (&'a MeasureDecisionGroupCapture, &'a MeasureGroupOrigin)>,
    decisions: BTreeMap<SourceId, SourceId>,
    measures: BTreeMap<(SourceId, u32), SourceId>,
}

impl<'a> ReadInventory<'a> {
    pub fn add(&mut self, row: &'a MeasureDecisionStoredOperation) -> Result<(), ApplicationError> {
        for ancestor in &row.measure_history.groups {
            self.group(&ancestor.capture, &ancestor.origin)?;
        }
        self.group(&row.group, &row.origin)
    }

    fn group(
        &mut self,
        group: &'a MeasureDecisionGroupCapture,
        origin: &'a MeasureGroupOrigin,
    ) -> Result<(), ApplicationError> {
        let owner = *origin.operation_id.as_uuid().as_bytes();
        if self
            .groups
            .insert(owner, (group, origin))
            .is_some_and(|old| old != (group, origin))
        {
            return Err(invalid(
                "read page has contradictory measure group evidence",
            ));
        }
        let decision = *origin.decision_id.as_uuid().as_bytes();
        if self
            .decisions
            .insert(decision, owner)
            .is_some_and(|old| old != owner)
        {
            return Err(invalid("read page has ambiguous decision ownership"));
        }
        for measure in &group.measures {
            let key = (
                *measure.result.id.as_uuid().as_bytes(),
                measure.result.revision.get(),
            );
            if self
                .measures
                .insert(key, owner)
                .is_some_and(|old| old != owner)
            {
                return Err(invalid(
                    "read page has ambiguous measure revision ownership",
                ));
            }
        }
        add_measure_group_sources(&mut self.sources, &group.review)
    }
}
