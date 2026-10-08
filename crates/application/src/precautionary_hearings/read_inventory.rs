use super::{
    source_inventory::SourceInventory, workflow_evidence::invalid,
    PrecautionaryHearingStoredOperation,
};
use crate::{
    precautionary_measures::{add_measure_group_sources, MeasureGroupEvidence},
    ApplicationError,
};
use std::collections::BTreeMap;
type SourceId = [u8; 16];

/// Independent valid histories must also agree where their immutable identities overlap.
#[derive(Default)]
pub(super) struct ReadInventory<'a> {
    sources: SourceInventory<'a>,
    groups: BTreeMap<SourceId, &'a MeasureGroupEvidence>,
    decisions: BTreeMap<SourceId, SourceId>,
    measures: BTreeMap<(SourceId, u32), SourceId>,
}

impl<'a> ReadInventory<'a> {
    pub fn add(
        &mut self,
        row: &'a PrecautionaryHearingStoredOperation,
    ) -> Result<(), ApplicationError> {
        for group in &row.history.measure_history.groups {
            let owner = *group.origin.operation_id.as_uuid().as_bytes();
            if self
                .groups
                .insert(owner, group)
                .is_some_and(|old| old != group)
            {
                return Err(invalid(
                    "read page has contradictory measure group evidence",
                ));
            }
            let decision = *group.origin.decision_id.as_uuid().as_bytes();
            if self
                .decisions
                .insert(decision, owner)
                .is_some_and(|old| old != owner)
            {
                return Err(invalid("read page has ambiguous decision ownership"));
            }
            for measure in &group.capture.measures {
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
            add_measure_group_sources(&mut self.sources, &group.capture.review)?;
        }
        for capture in &row.history.captures {
            self.sources.capture(capture)?;
        }
        Ok(())
    }
}
