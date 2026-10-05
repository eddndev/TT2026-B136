use super::{workflow_evidence::invalid, *};
use crate::{
    precautionary_hearings::source_inventory::SourceInventory, precautionary_measures::*,
    ApplicationError,
};
use std::collections::BTreeMap;
type Id = [u8; 16];

#[derive(PartialEq)]
enum Owner<'a> {
    Judicial(&'a MeasureDecisionGroupCapture, &'a MeasureGroupOrigin),
    Decision(&'a MeasureDecisionGroupCaptureV2, &'a MeasureGroupOrigin),
    Administrative(
        &'a MeasureAdministrativeCapture,
        &'a MeasureAdministrativeOrigin,
    ),
}

/// Compare individually validated page closures without imposing a new page-wide
/// proof budget. Shared complete owners and sources must retain exact equality.
#[derive(Default)]
pub(super) struct ReadInventory<'a> {
    sources: SourceInventory<'a>,
    owners: BTreeMap<Id, Owner<'a>>,
    decisions: BTreeMap<Id, Id>,
    members: BTreeMap<(Id, u32), Id>,
}
impl<'a> ReadInventory<'a> {
    pub fn add(
        &mut self,
        row: &'a MeasureAdministrativeStoredOperation,
    ) -> Result<(), ApplicationError> {
        self.add_history(&row.record_history)?;
        self.administrative(&row.capture, &row.origin)
    }
    pub fn add_history(
        &mut self,
        history: &'a MeasureDecisionRecordHistoryEvidence,
    ) -> Result<(), ApplicationError> {
        for g in &history.records.judicial.groups {
            self.judicial(&g.capture, &g.origin)?;
        }
        for g in &history.decisions {
            self.decision(&g.capture, &g.origin)?;
        }
        for a in &history.records.administrative {
            self.administrative(&a.capture, &a.origin)?;
        }
        Ok(())
    }

    fn owner(&mut self, id: Id, value: Owner<'a>) -> Result<(), ApplicationError> {
        if let Some(old) = self.owners.get(&id) {
            if *old != value {
                return Err(invalid("page operation owners differ"));
            }
        } else {
            self.owners.insert(id, value);
        }
        Ok(())
    }
    fn decision_id(&mut self, origin: &MeasureGroupOrigin) -> Result<Id, ApplicationError> {
        let id = *origin.operation_id.as_uuid().as_bytes();
        if self
            .decisions
            .insert(*origin.decision_id.as_uuid().as_bytes(), id)
            .is_some_and(|old| old != id)
        {
            return Err(invalid("page decision ownership differs"));
        }
        Ok(id)
    }
    fn member(&mut self, id: Id, revision: u32, owner: Id) -> Result<(), ApplicationError> {
        if self
            .members
            .insert((id, revision), owner)
            .is_some_and(|old| old != owner)
        {
            return Err(invalid("page record ownership differs"));
        }
        Ok(())
    }
    fn judicial(
        &mut self,
        g: &'a MeasureDecisionGroupCapture,
        origin: &'a MeasureGroupOrigin,
    ) -> Result<(), ApplicationError> {
        let owner = self.decision_id(origin)?;
        self.owner(owner, Owner::Judicial(g, origin))?;
        for m in &g.measures {
            self.member(
                *m.result.id.as_uuid().as_bytes(),
                m.result.revision.get(),
                owner,
            )?;
        }
        add_measure_group_sources(&mut self.sources, &g.review)
    }
    fn decision(
        &mut self,
        g: &'a MeasureDecisionGroupCaptureV2,
        origin: &'a MeasureGroupOrigin,
    ) -> Result<(), ApplicationError> {
        let owner = self.decision_id(origin)?;
        self.owner(owner, Owner::Decision(g, origin))?;
        for m in &g.measures {
            self.member(
                *m.result.id.as_uuid().as_bytes(),
                m.result.revision.get(),
                owner,
            )?;
        }
        add_record_decision_sources(&mut self.sources, &g.review)
    }
    fn administrative(
        &mut self,
        a: &'a MeasureAdministrativeCapture,
        origin: &'a MeasureAdministrativeOrigin,
    ) -> Result<(), ApplicationError> {
        let owner = *origin.operation_id.as_uuid().as_bytes();
        self.owner(owner, Owner::Administrative(a, origin))?;
        for m in &a.records {
            self.member(
                *m.result.id.as_uuid().as_bytes(),
                m.result.revision.get(),
                owner,
            )?;
        }
        super::record_history::add_sources(&mut self.sources, &a.review)
    }
}
